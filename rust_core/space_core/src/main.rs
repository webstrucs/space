// space/rust_core/space_core/src/main.rs

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use anyhow::Context;
use tracing::{info, error, Level, debug, warn};
use tracing_subscriber::FmtSubscriber;
use rand::rngs::OsRng;
use rand::Rng;
use tokio::sync::{mpsc, broadcast};
use std::time::SystemTime;
use tokio_util::sync::CancellationToken;

mod handlers;
mod metrics;
mod tls;
mod packets;
mod messages;
mod ipc;

use messages::{
    Message, ComponentState, ComponentMetrics, ErrorSeverity,
    ComponentConfig, ResourceLimits,
    message, control_command
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configuração do logger (tracing)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting space_core server...");

    // Inicializar o provedor criptográfico antes de usar TLS
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install default crypto provider"))?;
    info!("Crypto provider initialized.");

    // Canais para comunicação interna entre componentes
    let (command_tx, mut command_rx) = mpsc::channel::<Message>(100);
    let (status_tx, mut status_rx) = mpsc::channel::<Message>(100); 

    // Canais IPC dedicados para comunicação com a camada Python
    let (ipc_command_tx, mut ipc_command_rx) = mpsc::channel::<Message>(100);
    let (ipc_status_broadcast_tx, ipc_status_broadcast_rx) = broadcast::channel::<Message>(100); 

    // --- CancellationToken para Graceful Shutdown ---
    let shutdown_token_root = CancellationToken::new(); // Token ORIGINAL, usado APENAS para clonar
    // Clones para as tarefas que precisam MONITORAR o shutdown
    let shutdown_token_for_comp_mgr_monitor = shutdown_token_root.clone();
    let shutdown_token_for_status_mon_monitor = shutdown_token_root.clone();
    let shutdown_token_for_startup_cmds_monitor = shutdown_token_root.clone();
    let shutdown_token_for_packet_proc_monitor = shutdown_token_root.clone();
    let shutdown_token_for_ipc_server_monitor = shutdown_token_root.clone();
    let shutdown_token_for_http_listener_monitor = shutdown_token_root.clone();
    let shutdown_token_for_https_listener_monitor = shutdown_token_root.clone();
    let shutdown_token_for_select_final_monitor = shutdown_token_root.clone();

    // Clones para as tarefas que PRECISAM INICIAR o shutdown (chamar .cancel())
    let shutdown_token_for_comp_mgr_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento interno do manager
    let shutdown_token_for_ctrl_c_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento do Ctrl+C
    let shutdown_token_for_http_fail_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento em falha http
    let shutdown_token_for_https_fail_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento em falha https
    let shutdown_token_for_comp_mgr_fail_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento em falha do manager
    let shutdown_token_for_status_mon_fail_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento em falha do monitor
    let shutdown_token_for_ipc_server_fail_cancel = shutdown_token_root.clone(); // <--- NOVO: para o cancelamento em falha ipc


    // 1. Inicializa as métricas Prometheus
    metrics::init_metrics().await
        .context("Failed to initialize metrics recorder")?;
    info!("Metrics initialized and listening on 0.0.0.0:9000");

    // 2. Carrega a configuração TLS
    let tls_config = tls::load_server_config(
        "certs/cert.pem",
        "certs/key.pem",
    ).await.context("Failed to load TLS configuration")?;

    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));
    info!("TLS configuration loaded.");

    // --- CLONAGEM DE CANAIS PARA CADA TAREFA SPAWNADA ---
    // Clones para o component_manager
    let status_tx_for_comp_mgr = status_tx.clone();
    let ipc_status_broadcast_tx_for_comp_mgr = ipc_status_broadcast_tx.clone();

    // Clone para o status_monitor (para enviar erros internos)
    let status_tx_for_status_mon_errors = status_tx.clone();

    // Clones para a tarefa de comandos de startup/simulação
    let command_tx_for_startup = command_tx.clone();
    let ipc_status_broadcast_tx_for_startup = ipc_status_broadcast_tx.clone();

    // Clones para a tarefa de processamento de pacotes RAW
    let status_tx_for_packets_proc = status_tx.clone();
    let ipc_status_broadcast_tx_for_packets_proc = ipc_status_broadcast_tx.clone();
    // --- FIM DA CLONAGEM DE CANAIS ---


    // 3. Sistema de controle de componentes
    let component_manager = tokio::spawn(async move {
        let mut http_running = false;
        let mut https_running = false;
        let mut packet_processor_running = false;

        loop {
            tokio::select! {
                // Prioriza o sinal de shutdown
                _ = shutdown_token_for_comp_mgr_monitor.cancelled() => { // USADO PARA MONITORAR
                    info!("Component manager received shutdown signal. Exiting loop.");
                    break;
                }
                // Prioriza comandos vindos do IPC (Python)
                ipc_cmd_msg = ipc_command_rx.recv() => {
                    if let Some(msg) = ipc_cmd_msg {
                        if let Some(message::MessageType::Command(cmd)) = msg.message_type {
                            info!("Comando recebido via IPC: {:?}", cmd);
                            match cmd.command_type {
                                Some(control_command::CommandType::StartComponent(name)) => {
                                    info!("Starting component: {}", name);
                                    match name.as_str() {
                                        "http_server" => {
                                            http_running = true;
                                            let status = Message::new_status(
                                                "http_server".to_string(), true, 0, 0.1,
                                                "HTTP server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for http_server start") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for http_server start") { error!("{:?}", e); }
                                        }
                                        "https_server" => {
                                            https_running = true;
                                            let status = Message::new_status(
                                                "https_server".to_string(), true, 0, 0.1,
                                                "HTTPS server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for https_server start") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for https_server start") { error!("{:?}", e); }
                                        }
                                        "packet_processor" => {
                                            packet_processor_running = true;
                                            let status = Message::new_status(
                                                "packet_processor".to_string(), true, 0, 0.3,
                                                "Packet processor started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for packet_processor start") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for packet_processor start") { error!("{:?}", e); }
                                        }
                                        _ => {
                                            error!("Unknown component received via IPC: {}", name);
                                            let error_msg = Message::new_error(
                                                404, format!("Component '{}' not found via IPC.", name),
                                                "component_manager".to_string(), ErrorSeverity::Warning,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for unknown component (IPC)") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for unknown component") { error!("{:?}", e); }
                                        }
                                    }
                                }
                                Some(control_command::CommandType::StopComponent(name)) => {
                                    info!("Stopping component via IPC: {}", name);
                                    match name.as_str() {
                                        "http_server" => http_running = false,
                                        "https_server" => https_running = false,
                                        "packet_processor" => packet_processor_running = false,
                                        _ => error!("Unknown component to stop via IPC: {}", name),
                                    }
                                    let status = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} stopped via IPC.", name), ComponentState::Stopped, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for component stop (IPC)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for component stop") { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::RestartComponent(name)) => {
                                    info!("Restarting component via IPC: {}", name);
                                    let status_restarting = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} restarting via IPC...", name), ComponentState::Restarting, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_restarting.clone()).await.context("Failed to send internal status for component restarting (IPC)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_restarting).context("Failed to broadcast IPC status for component restarting") { error!("{:?}", e); }
                                    
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    
                                    let status_restarted = Message::new_status(
                                        name.clone(), true, 0, 0.1,
                                        format!("Component {} restarted via IPC.", name), ComponentState::Running, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_restarted.clone()).await.context("Failed to send internal status for component restarted (IPC)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_restarted).context("Failed to broadcast IPC status for component restarted") { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::RequestStatus(_)) => {
                                    info!("Global status requested via IPC");
                                    let components = vec![
                                        ("http_server", http_running),
                                        ("https_server", https_running),
                                        ("packet_processor", packet_processor_running),
                                    ];
                                    for (name, running) in components {
                                        let status_msg = Message::new_status(
                                            name.to_string(), running,
                                            if running { rand::random::<u64>() % 3600 } else { 0 },
                                            if running { rand::random::<f32>() * 0.8 } else { 0.0 },
                                            if running { "Running normally".to_string() } else { "Stopped".to_string() },
                                            if running { ComponentState::Running } else { ComponentState::Stopped },
                                            None,
                                        );
                                        if let Err(e) = status_tx_for_comp_mgr.send(status_msg.clone()).await.context(format!("Failed to send internal status for {} (IPC global status)", name)) { error!("{:?}", e); }
                                        if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_msg).context(format!("Failed to broadcast IPC status for {} (IPC global status)", name)) { error!("{:?}", e); }
                                    }
                                }
                                Some(control_command::CommandType::RequestComponentStatus(name)) => {
                                    info!("Status requested for component {} via IPC", name);
                                    let (running, current_load, uptime, msg, state, metrics_option) = match name.as_str() {
                                        "http_server" => (http_running, 0.1, rand::random::<u64>() % 3600, "HTTP server status".to_string(), if http_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                                        "https_server" => (https_running, 0.1, rand::random::<u64>() % 3600, "HTTPS server status".to_string(), if https_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                                        "packet_processor" => {
                                            let proc_metrics = ComponentMetrics {
                                                cpu_usage: 0.5,
                                                memory_usage_mb: 512,
                                                active_connections: 10,
                                                total_requests: 0,
                                                error_count: 0,
                                                requests_per_second: 0.0,
                                            };
                                            (packet_processor_running, 0.3, rand::random::<u64>() % 3600, "Packet processor status".to_string(), if packet_processor_running { ComponentState::Running } else { ComponentState::Stopped }, Some(proc_metrics))
                                        },
                                        _ => (false, 0.0, 0, "Unknown component".to_string(), ComponentState::Unknown, None),
                                    };
                                    let status_msg = Message::new_status(
                                        name.clone(), running, uptime, current_load, msg, state, metrics_option,
                                    );
                                    let component_name_for_log = if let Some(message::MessageType::Status(s)) = &status_msg.message_type {
                                        s.component_name.clone()
                                    } else {
                                        "Unknown Component".to_string()
                                    };
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_msg.clone()).await.context(format!("Failed to send internal status for {} (IPC component status)", component_name_for_log)) { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_msg).context(format!("Failed to broadcast IPC status for {} (IPC component status)", component_name_for_log)) { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::ConfigureComponent(data)) => {
                                    info!("Configuring component {} via IPC: {:?}", data.name, data.config);
                                    let status_configured = Message::new_status(
                                        data.name.clone(), true, 0, 0.0,
                                        format!("Component {} configured via IPC.", data.name), ComponentState::Running, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_configured.clone()).await.context(format!("Failed to send internal status for {} (IPC configure)", data.name)) { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_configured).context(format!("Failed to broadcast IPC status for {} (IPC configure)", data.name)) { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::Shutdown(_)) => {
                                    info!("Shutdown command received via IPC. Signalling global shutdown.");
                                    shutdown_token_for_comp_mgr_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
                                    break;
                                }
                                None => {
                                    error!("Received IPC command with no specific command type.");
                                    let error_msg = Message::new_error(
                                        400, "Internal command has no type.".to_string(),
                                        "component_manager".to_string(), ErrorSeverity::Warning,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for no IPC command type") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for no IPC command type") { error!("{:?}", e); }
                                }
                            }
                        } else {
                            warn!("Received non-command message via IPC in component manager: {:?}", msg);
                            let error_msg = Message::new_error(
                                400, format!("Unexpected message type in IPC: {:?}", msg.message_type),
                                "component_manager".to_string(), ErrorSeverity::Warning,
                            );
                            if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for unexpected IPC message type") { error!("{:?}", e); }
                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for unexpected IPC message type") { error!("{:?}", e); }
                        }
                    } else {
                        info!("IPC command channel closed, component manager exiting.");
                        break;
                    }
                }
                
                // Processa comandos internos (do startup/simulação)
                internal_cmd_msg = command_rx.recv() => {
                    if let Some(message) = internal_cmd_msg {
                        if let Some(message::MessageType::Command(cmd)) = message.message_type {
                            info!("Comando recebido internamente: {:?}", cmd);
                            match cmd.command_type {
                                Some(control_command::CommandType::StartComponent(name)) => {
                                    info!("Starting component internally: {}", name);
                                    match name.as_str() {
                                        "http_server" => {
                                            http_running = true;
                                            let status = Message::new_status(
                                                "http_server".to_string(), true, 0, 0.1,
                                                "HTTP server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for http_server start (internal)") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for http_server start (internal)") { error!("{:?}", e); }
                                        }
                                        "https_server" => {
                                            https_running = true;
                                            let status = Message::new_status(
                                                "https_server".to_string(), true, 0, 0.1,
                                                "HTTPS server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for https_server start (internal)") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for https_server start (internal)") { error!("{:?}", e); }
                                        }
                                        "packet_processor" => {
                                            packet_processor_running = true;
                                            let status = Message::new_status(
                                                "packet_processor".to_string(), true, 0, 0.3,
                                                "Packet processor started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for packet_processor start (internal)") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for packet_processor start (internal)") { error!("{:?}", e); }
                                        }
                                        _ => {
                                            error!("Unknown component internally: {}", name);
                                            let error_msg = Message::new_error(
                                                404, format!("Component '{}' not found internally.", name),
                                                "component_manager".to_string(), ErrorSeverity::Warning,
                                            );
                                            if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for unknown component (internal)") { error!("{:?}", e); }
                                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for unknown component (internal)") { error!("{:?}", e); }
                                        }
                                    }
                                }
                                Some(control_command::CommandType::StopComponent(name)) => {
                                    info!("Stopping component internally: {}", name);
                                    match name.as_str() {
                                        "http_server" => http_running = false,
                                        "https_server" => https_running = false,
                                        "packet_processor" => packet_processor_running = false,
                                        _ => error!("Unknown component to stop internally: {}", name),
                                    }
                                    let status = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} stopped internally.", name), ComponentState::Stopped, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status.clone()).await.context("Failed to send internal status for component stop (internal)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status).context("Failed to broadcast IPC status for component stop (internal)") { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::RestartComponent(name)) => {
                                    info!("Restarting component internally: {}", name);
                                    let status_restarting = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} restarting internally...", name), ComponentState::Restarting, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_restarting.clone()).await.context("Failed to send internal status for component restarting (internal)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_restarting).context("Failed to broadcast IPC status for component restarting (internal)") { error!("{:?}", e); }
                                    
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    
                                    let status_restarted = Message::new_status(
                                        name.clone(), true, 0, 0.1,
                                        format!("Component {} restarted internally.", name), ComponentState::Running, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_restarted.clone()).await.context("Failed to send internal status for component restarted (internal)") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_restarted).context("Failed to broadcast IPC status for component restarted (internal)") { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::RequestStatus(_)) => {
                                    info!("Global status requested internally");
                                    let components = vec![
                                        ("http_server", http_running),
                                        ("https_server", https_running),
                                        ("packet_processor", packet_processor_running),
                                    ];
                                    for (name, running) in components {
                                        let status_msg = Message::new_status(
                                            name.to_string(), running,
                                            if running { rand::random::<u64>() % 3600 } else { 0 },
                                            if running { rand::random::<f32>() * 0.8 } else { 0.0 },
                                            if running { "Running normally".to_string() } else { "Stopped".to_string() },
                                            if running { ComponentState::Running } else { ComponentState::Stopped },
                                            None,
                                        );
                                        if let Err(e) = status_tx_for_comp_mgr.send(status_msg.clone()).await.context(format!("Failed to send internal status for {} (internal global status)", name)) { error!("{:?}", e); }
                                        if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_msg).context(format!("Failed to broadcast IPC status for {} (internal global status)", name)) { error!("{:?}", e); }
                                    }
                                }
                                Some(control_command::CommandType::RequestComponentStatus(name)) => {
                                    info!("Status requested for component {} internally", name);
                                    let (running, current_load, uptime, msg, state, metrics_option) = match name.as_str() {
                                        "http_server" => (http_running, 0.1, rand::random::<u64>() % 3600, "HTTP server status".to_string(), if http_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                                        "https_server" => (https_running, 0.1, rand::random::<u64>() % 3600, "HTTPS server status".to_string(), if https_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                                        "packet_processor" => {
                                            let proc_metrics = ComponentMetrics {
                                                cpu_usage: 0.5,
                                                memory_usage_mb: 512,
                                                active_connections: 10,
                                                total_requests: 0,
                                                error_count: 0,
                                                requests_per_second: 0.0,
                                            };
                                            (packet_processor_running, 0.3, rand::random::<u64>() % 3600, "Packet processor status".to_string(), if packet_processor_running { ComponentState::Running } else { ComponentState::Stopped }, Some(proc_metrics))
                                        },
                                        _ => (false, 0.0, 0, "Unknown component".to_string(), ComponentState::Unknown, None),
                                    };
                                    let status_msg = Message::new_status(
                                        name.clone(), running, uptime, current_load, msg, state, metrics_option,
                                    );
                                    let component_name_for_log = if let Some(message::MessageType::Status(s)) = &status_msg.message_type {
                                        s.component_name.clone()
                                    } else {
                                        "Unknown Component".to_string()
                                    };
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_msg.clone()).await.context(format!("Failed to send internal status for {} (internal component status)", component_name_for_log)) { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_msg).context(format!("Failed to broadcast IPC status for {} (internal component status)", component_name_for_log)) { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::ConfigureComponent(data)) => {
                                    info!("Configuring component internally {}: {:?}", data.name, data.config);
                                    let status_configured = Message::new_status(
                                        data.name.clone(), true, 0, 0.0,
                                        format!("Component {} configured internally.", data.name), ComponentState::Running, None,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(status_configured.clone()).await.context(format!("Failed to send internal status for {} (internal configure)", data.name)) { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(status_configured).context(format!("Failed to broadcast IPC status for {} (internal configure)", data.name)) { error!("{:?}", e); }
                                }
                                Some(control_command::CommandType::Shutdown(_)) => {
                                    info!("Internal Shutdown command received. Signalling global shutdown.");
                                    shutdown_token_for_comp_mgr_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
                                    break;
                                }
                                None => {
                                    error!("Received internal command with no specific command type.");
                                    let error_msg = Message::new_error(
                                        400, "Internal command has no type.".to_string(),
                                        "component_manager".to_string(), ErrorSeverity::Warning,
                                    );
                                    if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for no internal command type") { error!("{:?}", e); }
                                    if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for no internal command type") { error!("{:?}", e); }
                                }
                            }
                        } else {
                            warn!("Received non-command message internally in component manager: {:?}", message);
                            let error_msg = Message::new_error(
                                400, format!("Unexpected message type internally: {:?}", message.message_type),
                                "component_manager".to_string(), ErrorSeverity::Warning,
                            );
                            if let Err(e) = status_tx_for_comp_mgr.send(error_msg.clone()).await.context("Failed to send internal error for unexpected internal message type") { error!("{:?}", e); }
                            if let Err(e) = ipc_status_broadcast_tx_for_comp_mgr.send(error_msg).context("Failed to broadcast IPC error for unexpected internal message type") { error!("{:?}", e); }
                        }
                    } else {
                        info!("Internal command channel closed, component manager exiting.");
                        break;
                    }
                }
            }
        }
    });

    // Monitor de status (recebe de status_rx para logar)
    let status_monitor = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Prioriza o sinal de shutdown
                _ = shutdown_token_for_status_mon_monitor.cancelled() => {
                    info!("Status monitor received shutdown signal. Exiting loop.");
                    break;
                }
                message = status_rx.recv() => {
                    if let Some(message) = message {
                        match message.message_type {
                            Some(message::MessageType::Status(status)) => {
                                info!(
                                    "Component Status - {}: {} (Load: {:.2}, Uptime: {}s) - {} - State: {:?} - Timestamp: {}",
                                    status.component_name,
                                    if status.is_running { "RUNNING" } else { "STOPPED" },
                                    status.current_load,
                                    status.uptime_seconds,
                                    status.message,
                                    ComponentState::try_from(status.component_state).unwrap_or(ComponentState::Unknown),
                                    status.timestamp
                                );
                            }
                            Some(message::MessageType::Error(error_msg)) => {
                                error!(
                                    "ERROR - Source: {} - Code: {} - Severity: {:?} - Description: {} - Timestamp: {}",
                                    error_msg.source_component,
                                    error_msg.error_code,
                                    ErrorSeverity::try_from(error_msg.severity).unwrap_or(ErrorSeverity::Info),
                                    error_msg.description,
                                    error_msg.timestamp
                                );
                            }
                            Some(message::MessageType::HeartbeatTimestamp(timestamp)) => {
                                debug!("Received Heartbeat at: {}", timestamp);
                            }
                            Some(message::MessageType::AckMessageId(message_id)) => {
                                debug!("Received Acknowledge for message ID: {}", message_id);
                            }
                            Some(message::MessageType::Command(_)) => {
                                warn!("Received Command message in status monitor (unexpected): {:?}", message);
                                let error_msg = Message::new_error(
                                    400, "Unexpected command message received in status monitor.".to_string(),
                                    "status_monitor".to_string(), ErrorSeverity::Warning,
                                );
                                if let Err(e) = status_tx_for_status_mon_errors.send(error_msg).await.context("Failed to send internal error for unexpected command in status monitor") { error!("{:?}", e); }
                            }
                            None => {
                                warn!("Received empty message type in status monitor: {:?}", message);
                                let error_msg = Message::new_error(
                                    400, "Empty message type received in status monitor.".to_string(),
                                    "status_monitor".to_string(), ErrorSeverity::Warning,
                                );
                                if let Err(e) = status_tx_for_status_mon_errors.send(error_msg).await.context("Failed to send internal error for empty message type in status monitor") { error!("{:?}", e); }
                            }
                        }
                    } else {
                        info!("Status channel closed, status monitor exiting.");
                        break;
                    }
                }
            }
        }
        info!("Status monitor task finished.");
    });

    // 4. Configura os listeners de rede
    let http_listener = TcpListener::bind("0.0.0.0:8080").await
        .context("Failed to bind HTTP listener to 0.0.0.0:8080")?;
    info!("HTTP listener bound to 0.0.0.0:8080");

    let https_listener = TcpListener::bind("0.0.0.0:8443").await
        .context("Failed to bind HTTPS listener to 0.0.0.0:8443")?;
    info!("HTTPS listener bound to 0.0.0.0:8443");

    // Iniciar o servidor IPC, passando o broadcast receiver para status e o token de shutdown
    let ipc_server_handle = tokio::spawn(async move {
        if let Err(e) = ipc::start_ipc_server(ipc_status_broadcast_rx, ipc_command_tx, shutdown_token_for_ipc_server_monitor).await {
            error!("IPC Server failed: {:?}", e);
        }
        info!("Servidor IPC encerrado.");
    });

    // Iniciar componentes automaticamente e enviar comandos de teste
    tokio::spawn(async move {
        tokio::select! {
            _ = shutdown_token_for_startup_cmds_monitor.cancelled() => {
                info!("Startup commands task received shutdown signal. Exiting loop.");
            }
            _ = async { // Este bloco async{} é o Future que o select vai esperar
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                info!("Sending startup commands to component manager...");
                
                if let Err(e) = command_tx_for_startup.send(Message::new_start_component_command("http_server".to_string())).await.context("Failed to send startup command: http_server") { error!("{:?}", e); }
                if let Err(e) = command_tx_for_startup.send(Message::new_start_component_command("https_server".to_string())).await.context("Failed to send startup command: https_server") { error!("{:?}", e); }
                if let Err(e) = command_tx_for_startup.send(Message::new_start_component_command("packet_processor".to_string())).await.context("Failed to send startup command: packet_processor") { error!("{:?}", e); }
                
                // Exemplo de comando de configuração
                let mut settings = std::collections::HashMap::new();
                settings.insert("max_connections".to_string(), "200".to_string());
                settings.insert("buffer_size".to_string(), "8192".to_string());
                let config_cmd = Message::new_configure_component_command(
                    "http_server".to_string(),
                    ComponentConfig {
                        settings,
                        log_level: Some("INFO".to_string()),
                        max_resources: Some(ResourceLimits {
                            max_cpu: Some(0.7),
                            max_memory_mb: Some(512),
                            max_connections: None,
                        }),
                    },
                );
                if let Err(e) = command_tx_for_startup.send(config_cmd).await.context("Failed to send startup command: configure http_server") { error!("{:?}", e); }

                // Requisitar status a cada 30 segundos
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    info!("Sending RequestStatus command...");
                    if let Err(e) = command_tx_for_startup.send(Message::new_request_status_command()).await.context("Failed to send periodic RequestStatus command") { error!("{:?}", e); }
                    
                    // Simular o envio de um heartbeat
                    if let Err(e) = ipc_status_broadcast_tx_for_startup.send(Message::new_heartbeat()).context("Failed to broadcast periodic Heartbeat") { error!("{:?}", e); }
                }
            } => {}
        }
        info!("Startup commands task finished.");
    });

    // 5. Spawna tarefas para lidar com conexões HTTP
    let http_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Prioriza o sinal de shutdown
                _ = shutdown_token_for_http_listener_monitor.cancelled() => { // USANDO O CLONE DE MONITORAMENTO
                    info!("HTTP listener received shutdown signal. Stopping accepting new connections.");
                    break;
                }
                accept_result = http_listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            info!("Accepted HTTP connection from: {}", peer_addr);
                            tokio::spawn(async move {
                                if let Err(e) = handlers::handle_http_connection(stream, peer_addr).await.context(format!("HTTP handler for {} failed", peer_addr)) {
                                    error!("{:?}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Error accepting HTTP connection: {}", e);
                        }
                    }
                }
            }
        }
        info!("HTTP listener task finished.");
    });

    // 6. Spawna tarefas para lidar com conexões HTTPS
    let tls_acceptor_arc = Arc::new(tls_acceptor);
    let https_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Prioriza o sinal de shutdown
                _ = shutdown_token_for_https_listener_monitor.cancelled() => { // USANDO O CLONE DE MONITORAMENTO
                    info!("HTTPS listener received shutdown signal. Stopping accepting new connections.");
                    break;
                }
                accept_result = https_listener.accept() => {
                    let current_tls_acceptor = Arc::clone(&tls_acceptor_arc);
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            info!("Accepted HTTPS connection from: {}", peer_addr);
                            tokio::spawn(async move {
                                match current_tls_acceptor.accept(stream).await.context(format!("TLS handshake failed for {}", peer_addr)) {
                                    Ok(tls_stream_server) => {
                                        if let Err(e) = handlers::handle_https_connection(tokio_rustls::TlsStream::Server(tls_stream_server), peer_addr).await.context(format!("HTTPS handler for {} failed", peer_addr)) {
                                            error!("{:?}", e);
                                        }
                                    }
                                    Err(e) => {
                                        error!("{:?}", e);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            error!("Error accepting HTTPS connection: {}", e);
                        }
                    }
                }
            }
        }
        info!("HTTPS listener task finished.");
    });

    // Exemplo de loop para processamento de pacotes RAW (se habilitado ou configurado)
    tokio::spawn(async move {
        info!("Starting simulated RAW packet processing...");
        let mut rng = OsRng;
        let mut packets_processed_total = 0;
        let mut error_count_simulated = 0;
        let mut last_check_time = SystemTime::now();
        let mut packets_since_last_check = 0; 

        loop {
            tokio::select! {
                // Prioriza o sinal de shutdown
                _ = shutdown_token_for_packet_proc_monitor.cancelled() => {
                    info!("Packet processor received shutdown signal. Exiting loop.");
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => {
                    debug!("Simulating RAW packet reception...");

                    crate::metrics::RAW_PACKETS_TOTAL.increment(1);
                    packets_processed_total += 1;
                    packets_since_last_check += 1;

                    if rng.gen_bool(0.7) {
                        crate::metrics::RAW_IPV4_PACKETS_TOTAL.increment(1);
                        packets::process_packet_data(&[0x45, 0x00, 0x00, 0x34, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06, 0x7c, 0xb0, 0x7f, 0x00, 0x00, 0x01, 0x7f, 0x00, 0x00, 0x01]);
                    } else {
                        crate::metrics::RAW_IPV6_PACKETS_TOTAL.increment(1);
                        packets::process_packet_data(&[0x60, 0x00, 0x00, 0x00, 0x00, 0x14, 0x06, 0x40, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);
                    }

                    // Simular erros ocasionalmente
                    if rng.gen_bool(0.05) { // 5% de chance de erro
                        error_count_simulated += 1;
                        let error_msg = Message::new_error(
                            1001,
                            "Simulated packet processing error".to_string(),
                            "packet_processor".to_string(),
                            ErrorSeverity::Error,
                        );
                        if let Err(e) = status_tx_for_packets_proc.send(error_msg.clone()).await.context("Failed to send internal error for simulated packet error") { error!("{:?}", e); }
                        if let Err(e) = ipc_status_broadcast_tx_for_packets_proc.send(error_msg).context("Failed to broadcast IPC error for simulated packet error") { error!("{:?}", e); }
                    }

                    // Calcular RPS simulado
                    let now = SystemTime::now();
                    let elapsed_seconds = now.duration_since(last_check_time).unwrap_or_default().as_secs_f32();
                    let rps = if elapsed_seconds > 0.0 { packets_since_last_check as f32 / elapsed_seconds } else { 0.0 };

                    if elapsed_seconds >= 5.0 { // Recalcular RPS a cada 5 segundos
                        last_check_time = now;
                        packets_since_last_check = 0;
                    }

                    // Enviar um status simulado para o monitor
                    let metrics = ComponentMetrics {
                        cpu_usage: rng.gen::<f32>() * 0.5 + 0.1,
                        memory_usage_mb: rng.gen::<u64>() % 100 + 100,
                        active_connections: rng.gen::<u32>() % 20,
                        total_requests: packets_processed_total,
                        error_count: error_count_simulated,
                        requests_per_second: rps,
                    };
                    let status = Message::new_status(
                        "packet_processor".to_string(),
                        true,
                        packets_processed_total as u64 * 3, // Tempo aproximado
                        rng.gen::<f32>() * 0.5,
                        "Processing packets".to_string(),
                        ComponentState::Running,
                        Some(metrics),
                    );

                    if let Err(e) = status_tx_for_packets_proc.send(status.clone()).await.context("Failed to send internal status for packet processor simulation") { error!("{:?}", e); }
                    if let Err(e) = ipc_status_broadcast_tx_for_packets_proc.send(status).context("Failed to broadcast IPC status for packet processor simulation") { error!("{:?}", e); }
                }
            }
        }
        info!("Packet processor task finished.");
    });

    // Espera por Ctrl+C OU qualquer uma das tarefas principais falhar
    tokio::select! {
        _ = shutdown_token_for_select_final_monitor.cancelled() => { // USANDO O CLONE DE MONITORAMENTO
            info!("Global shutdown signal received (Ctrl+C or IPC Shutdown command).");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received. Initiating graceful shutdown.");
            shutdown_token_for_ctrl_c_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
        }
        _ = http_handle => {
            error!("HTTP listener task finished unexpectedly. This is a critical error. Initiating graceful shutdown.");
            shutdown_token_for_http_fail_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
            return Err(anyhow::anyhow!("HTTP listener task failed unexpectedly"));
        }
        _ = https_handle => {
            error!("HTTPS listener task finished unexpectedly. This is a critical error. Initiating graceful shutdown.");
            shutdown_token_for_https_fail_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
            return Err(anyhow::anyhow!("HTTPS listener task failed unexpectedly"));
        }
        _ = component_manager => {
            error!("Component manager finished unexpectedly. This is a critical error. Initiating graceful shutdown.");
            shutdown_token_for_comp_mgr_fail_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
            return Err(anyhow::anyhow!("Component manager failed unexpectedly"));
        }
        _ = status_monitor => {
            error!("Status monitor finished unexpectedly. This is a critical error. Initiating graceful shutdown.");
            shutdown_token_for_status_mon_fail_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
            return Err(anyhow::anyhow!("Status monitor failed unexpectedly"));
        }
        _ = ipc_server_handle => {
            error!("Servidor IPC encerrado inesperadamente. Este é um erro crítico. Iniciando graceful shutdown.");
            shutdown_token_for_ipc_server_fail_cancel.cancel(); // <--- USA O CLONE DE CANCELAMENTO
            return Err(anyhow::anyhow!("IPC server failed unexpectedly"));
        }
    }

    info!("All main tasks signaled for shutdown. Waiting for them to complete...");

    // Opcional: Esperar que todas as tarefas spawadas terminem.
    // Em um sistema real, você pode querer um timeout aqui.
    // http_handle.await?;
    // https_handle.await?;
    // component_manager.await?;
    // status_monitor.await?;
    // ipc_server_handle.await?;

    info!("Space Core server shutdown complete.");
    Ok(())
}
