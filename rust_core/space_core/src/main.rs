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

mod handlers;
mod metrics;
mod tls;
mod packets;
mod messages;
mod ipc;

// Importar os tipos de mensagem (agora vindos do Protobuf via messages.rs)
use messages::{
    Message, ComponentState, ComponentMetrics, ErrorSeverity,
    ComponentConfig, ResourceLimits, // Removido 'ConfigureComponentData' que era redundante aqui
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

    // 3. Sistema de controle de componentes
    let status_tx_clone = status_tx.clone();
    let ipc_status_broadcast_tx_clone = ipc_status_broadcast_tx.clone();

    let component_manager = tokio::spawn(async move {
        let mut http_running = false;
        let mut https_running = false;
        let mut packet_processor_running = false;

        loop {
            tokio::select! {
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
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status); 
                                        }
                                        "https_server" => {
                                            https_running = true;
                                            let status = Message::new_status(
                                                "https_server".to_string(), true, 0, 0.1,
                                                "HTTPS server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status);
                                        }
                                        "packet_processor" => {
                                            packet_processor_running = true;
                                            let status = Message::new_status(
                                                "packet_processor".to_string(), true, 0, 0.3,
                                                "Packet processor started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status);
                                        }
                                        _ => {
                                            error!("Unknown component received via IPC: {}", name);
                                            let error_msg = Message::new_error(
                                                404, format!("Component '{}' not found via IPC.", name),
                                                "component_manager".to_string(), ErrorSeverity::Warning,
                                            );
                                            let _ = status_tx_clone.send(error_msg.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(error_msg);
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
                                    let _ = status_tx_clone.send(status.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status);
                                }
                                Some(control_command::CommandType::RestartComponent(name)) => {
                                    info!("Restarting component via IPC: {}", name);
                                    let status_restarting = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} restarting via IPC...", name), ComponentState::Restarting, None,
                                    );
                                    let _ = status_tx_clone.send(status_restarting.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_restarting);
                                    
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    
                                    let status_restarted = Message::new_status(
                                        name.clone(), true, 0, 0.1,
                                        format!("Component {} restarted via IPC.", name), ComponentState::Running, None,
                                    );
                                    let _ = status_tx_clone.send(status_restarted.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_restarted);
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
                                        let _ = status_tx_clone.send(status_msg.clone()).await;
                                        let _ = ipc_status_broadcast_tx_clone.send(status_msg);
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
                                        name, running, uptime, current_load, msg, state, metrics_option,
                                    );
                                    let _ = status_tx_clone.send(status_msg.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_msg);
                                }
                                Some(control_command::CommandType::ConfigureComponent(data)) => {
                                    info!("Configuring component {} via IPC: {:?}", data.name, data.config);
                                    let status_configured = Message::new_status(
                                        data.name.clone(), true, 0, 0.0,
                                        format!("Component {} configured via IPC.", data.name), ComponentState::Running, None,
                                    );
                                    let _ = status_tx_clone.send(status_configured.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_configured);
                                }
                                Some(control_command::CommandType::Shutdown(_)) => {
                                    info!("Shutdown command received via IPC. Shutting down gracefully...");
                                    break;
                                }
                                None => {
                                    error!("Received IPC command with no specific command type.");
                                    let error_msg = Message::new_error(
                                        400, "IPC command has no type.".to_string(),
                                        "component_manager".to_string(), ErrorSeverity::Warning,
                                    );
                                    let _ = status_tx_clone.send(error_msg.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(error_msg);
                                }
                            }
                        } else {
                            warn!("Received non-command message via IPC in component manager: {:?}", msg);
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
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status);
                                        }
                                        "https_server" => {
                                            https_running = true;
                                            let status = Message::new_status(
                                                "https_server".to_string(), true, 0, 0.1,
                                                "HTTPS server started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status);
                                        }
                                        "packet_processor" => {
                                            packet_processor_running = true;
                                            let status = Message::new_status(
                                                "packet_processor".to_string(), true, 0, 0.3,
                                                "Packet processor started successfully".to_string(), ComponentState::Running, None,
                                            );
                                            let _ = status_tx_clone.send(status.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(status);
                                        }
                                        _ => {
                                            error!("Unknown component internally: {}", name);
                                            let error_msg = Message::new_error(
                                                404, format!("Component '{}' not found internally.", name),
                                                "component_manager".to_string(), ErrorSeverity::Warning,
                                            );
                                            let _ = status_tx_clone.send(error_msg.clone()).await;
                                            let _ = ipc_status_broadcast_tx_clone.send(error_msg);
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
                                    let _ = status_tx_clone.send(status.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status);
                                }
                                Some(control_command::CommandType::RestartComponent(name)) => {
                                    info!("Restarting component internally: {}", name);
                                    let status_restarting = Message::new_status(
                                        name.clone(), false, 0, 0.0,
                                        format!("Component {} restarting internally...", name), ComponentState::Restarting, None,
                                    );
                                    let _ = status_tx_clone.send(status_restarting.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_restarting);
                                    
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                    
                                    let status_restarted = Message::new_status(
                                        name.clone(), true, 0, 0.1,
                                        format!("Component {} restarted internally.", name), ComponentState::Running, None,
                                    );
                                    let _ = status_tx_clone.send(status_restarted.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_restarted);
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
                                        let _ = status_tx_clone.send(status_msg.clone()).await;
                                        let _ = ipc_status_broadcast_tx_clone.send(status_msg);
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
                                        name, running, uptime, current_load, msg, state, metrics_option,
                                    );
                                    let _ = status_tx_clone.send(status_msg.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_msg);
                                }
                                Some(control_command::CommandType::ConfigureComponent(data)) => {
                                    info!("Configuring component internally {}: {:?}", data.name, data.config);
                                    let status_configured = Message::new_status(
                                        data.name.clone(), true, 0, 0.0,
                                        format!("Component {} configured internally.", data.name), ComponentState::Running, None,
                                    );
                                    let _ = status_tx_clone.send(status_configured.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(status_configured);
                                }
                                Some(control_command::CommandType::Shutdown(_)) => {
                                    info!("Internal Shutdown command received. Shutting down gracefully...");
                                    break;
                                }
                                None => {
                                    error!("Received internal command with no specific command type.");
                                    let error_msg = Message::new_error(
                                        400, "Internal command has no type.".to_string(),
                                        "component_manager".to_string(), ErrorSeverity::Warning,
                                    );
                                    let _ = status_tx_clone.send(error_msg.clone()).await;
                                    let _ = ipc_status_broadcast_tx_clone.send(error_msg);
                                }
                            }
                        } else {
                            warn!("Received non-command message internally in component manager: {:?}", message);
                        }
                    } else {
                        info!("Internal command channel closed, component manager exiting.");
                        break;
                    }
                }
            }
        }
    });

    // Monitor de status (recebe de status_tx para logar)
    let status_monitor = tokio::spawn(async move {
        while let Some(message) = status_rx.recv().await {
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
                }
                None => {
                    warn!("Received empty message type in status monitor: {:?}", message);
                }
            }
        }
        info!("Status monitor channel closed, exiting.");
    });

    // 4. Configura os listeners de rede
    let http_listener = TcpListener::bind("0.0.0.0:8080").await
        .context("Failed to bind HTTP listener to 0.0.0.0:8080")?;
    info!("HTTP listener bound to 0.0.0.0:8080");

    let https_listener = TcpListener::bind("0.0.0.0:8443").await
        .context("Failed to bind HTTPS listener to 0.0.0.0:8443")?;
    info!("HTTPS listener bound to 0.0.0.0:8443");

    // Iniciar o servidor IPC, passando o broadcast receiver para status
    let ipc_server_handle = tokio::spawn(async move {
        if let Err(e) = ipc::start_ipc_server(ipc_status_broadcast_rx, ipc_command_tx).await {
            error!("Servidor IPC falhou: {:?}", e);
        }
        info!("Servidor IPC encerrado.");
    });

    // Iniciar componentes automaticamente e enviar comandos de teste
    let command_tx_startup_clone = command_tx.clone();
    let ipc_status_broadcast_tx_for_startup = ipc_status_broadcast_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        info!("Sending startup commands to component manager...");
        
        let _ = command_tx_startup_clone.send(Message::new_start_component_command("http_server".to_string())).await;
        let _ = command_tx_startup_clone.send(Message::new_start_component_command("https_server".to_string())).await;
        let _ = command_tx_startup_clone.send(Message::new_start_component_command("packet_processor".to_string())).await;
        
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
        let _ = command_tx_startup_clone.send(config_cmd).await;

        // Requisitar status a cada 30 segundos
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            info!("Sending RequestStatus command...");
            let _ = command_tx_startup_clone.send(Message::new_request_status_command()).await;
            
            // Simular o envio de um heartbeat
            let _ = ipc_status_broadcast_tx_for_startup.send(Message::new_heartbeat());
        }
    });

    // 5. Spawna tarefas para lidar com conexões HTTP
    let http_handle = tokio::spawn(async move {
        loop {
            match http_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("Accepted HTTP connection from: {}", peer_addr);
                    tokio::spawn(handlers::handle_http_connection(stream, peer_addr));
                }
                Err(e) => {
                    error!("Error accepting HTTP connection: {}", e);
                }
            }
        }
    });

    // 6. Spawna tarefas para lidar com conexões HTTPS
    let tls_acceptor_arc = Arc::new(tls_acceptor);
    let https_handle = tokio::spawn(async move {
        loop {
            let current_tls_acceptor = Arc::clone(&tls_acceptor_arc);
            match https_listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("Accepted HTTPS connection from: {}", peer_addr);
                    tokio::spawn(async move {
                        match current_tls_acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                handlers::handle_https_connection(tls_stream, peer_addr).await;
                            }
                            Err(e) => {
                                error!("TLS handshake failed for {}: {}", peer_addr, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting HTTPS connection: {}", e);
                }
            }
        }
    });

    // Exemplo de loop para processamento de pacotes RAW (se habilitado ou configurado)
    let status_tx_for_packets = status_tx.clone();
    let ipc_status_broadcast_tx_for_packets = ipc_status_broadcast_tx.clone();
    tokio::spawn(async move {
        info!("Starting simulated RAW packet processing...");
        let mut rng = OsRng;
        let mut packets_processed_total = 0;
        let mut error_count_simulated = 0;
        let mut last_check_time = SystemTime::now();
        let mut packets_since_last_check = 0;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
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
                let _ = status_tx_for_packets.send(error_msg.clone()).await;
                let _ = ipc_status_broadcast_tx_for_packets.send(error_msg);
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

            let _ = status_tx_for_packets.send(status.clone()).await;
            let _ = ipc_status_broadcast_tx_for_packets.send(status);
        }
    });

    // Espera por Ctrl+C para desligar o servidor
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl+C received, shutting down...");
        }
        _ = http_handle => {
            error!("HTTP listener task finished unexpectedly.");
        }
        _ = https_handle => {
            error!("HTTPS listener task finished unexpectedly.");
        }
        _ = component_manager => {
            error!("Component manager finished unexpectedly.");
        }
        _ = status_monitor => {
            error!("Status monitor finished unexpectedly.");
        }
        _ = ipc_server_handle => {
            error!("Servidor IPC encerrado inesperadamente.");
        }
    }

    Ok(())
}
