// space/rust_core/space_core/src/main.rs

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use anyhow::Context;
use tracing::{info, error, Level, debug, warn}; // Adicionado debug e warn
use tracing_subscriber::FmtSubscriber;
use rand::rngs::OsRng;
use rand::Rng;
use tokio::sync::mpsc;
use std::time::SystemTime; // Removido UNIX_EPOCH, pois só é usado em messages.rs

mod handlers;
mod metrics;
mod tls;
mod packets;
mod messages;

// Importar os tipos de mensagem.
use messages::{Message, ControlCommand, ComponentState, ComponentMetrics, ErrorSeverity};

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

    // Canal para comunicação entre componentes
    let (command_tx, mut command_rx) = mpsc::channel::<Message>(100);
    let (status_tx, mut status_rx) = mpsc::channel::<Message>(100);

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
    let component_manager = tokio::spawn(async move {
        let mut http_running = false;
        let mut https_running = false;
        let mut packet_processor_running = false;

        while let Some(message) = command_rx.recv().await {
            if let Message::Command(cmd) = message {
                match cmd {
                    ControlCommand::StartComponent(name) => {
                        info!("Starting component: {}", name);
                        match name.as_str() {
                            "http_server" => {
                                http_running = true;
                                let status = Message::new_status(
                                    "http_server".to_string(),
                                    true,
                                    0, // uptime_seconds
                                    0.1, // current_load
                                    "HTTP server started successfully".to_string(),
                                    ComponentState::Running,
                                    None, // Sem métricas para este início
                                );
                                let _ = status_tx_clone.send(status).await;
                            }
                            "https_server" => {
                                https_running = true;
                                let status = Message::new_status(
                                    "https_server".to_string(),
                                    true,
                                    0, // uptime_seconds
                                    0.1, // current_load
                                    "HTTPS server started successfully".to_string(),
                                    ComponentState::Running,
                                    None, // Sem métricas
                                );
                                let _ = status_tx_clone.send(status).await;
                            }
                            "packet_processor" => {
                                packet_processor_running = true;
                                let status = Message::new_status(
                                    "packet_processor".to_string(),
                                    true,
                                    0, // uptime_seconds
                                    0.3, // current_load
                                    "Packet processor started successfully".to_string(),
                                    ComponentState::Running,
                                    None, // Sem métricas
                                );
                                let _ = status_tx_clone.send(status).await;
                            }
                            _ => {
                                error!("Unknown component: {}", name);
                                let error_msg = Message::new_error(
                                    404,
                                    format!("Component '{}' not found for start command.", name),
                                    "component_manager".to_string(),
                                    ErrorSeverity::Warning,
                                );
                                let _ = status_tx_clone.send(error_msg).await;
                            }
                        }
                    }
                    ControlCommand::StopComponent(name) => {
                        info!("Stopping component: {}", name);
                        match name.as_str() {
                            "http_server" => http_running = false,
                            "https_server" => https_running = false,
                            "packet_processor" => packet_processor_running = false,
                            _ => {
                                error!("Unknown component: {}", name);
                                let error_msg = Message::new_error(
                                    404,
                                    format!("Component '{}' not found for stop command.", name),
                                    "component_manager".to_string(),
                                    ErrorSeverity::Warning,
                                );
                                let _ = status_tx_clone.send(error_msg).await;
                            }
                        }
                    }
                    ControlCommand::RestartComponent(name) => {
                        info!("Restarting component: {}", name);
                        // Lógica de restart (parar e iniciar)
                        let _ = status_tx_clone.send(Message::new_status(
                            name.clone(),
                            false, 0, 0.0,
                            format!("Component {} restarting...", name),
                            ComponentState::Restarting,
                            None,
                        )).await;
                        // Simulando um restart
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        let _ = status_tx_clone.send(Message::new_status(
                            name.clone(),
                            true, 0, 0.1,
                            format!("Component {} restarted.", name),
                            ComponentState::Running,
                            None,
                        )).await;
                    }
                    ControlCommand::RequestStatus => {
                        info!("Status requested for all components");
                        // Enviar status de todos os componentes
                        let components = vec![
                            ("http_server", http_running),
                            ("https_server", https_running),
                            ("packet_processor", packet_processor_running),
                        ];
                        
                        for (name, running) in components {
                            let status_msg = Message::new_status(
                                name.to_string(),
                                running,
                                if running { rand::random::<u64>() % 3600 } else { 0 },
                                if running { rand::random::<f32>() * 0.8 } else { 0.0 },
                                if running { "Running normally".to_string() } else { "Stopped".to_string() },
                                if running { ComponentState::Running } else { ComponentState::Stopped },
                                None, // Sem métricas para requisição geral
                            );
                            let _ = status_tx_clone.send(status_msg).await;
                        }
                    }
                    ControlCommand::RequestComponentStatus(name) => {
                        info!("Status requested for component: {}", name);
                        let (running, current_load, uptime, msg, state, metrics_option) = match name.as_str() {
                            "http_server" => (http_running, 0.1, rand::random::<u64>() % 3600, "HTTP server status".to_string(), if http_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                            "https_server" => (https_running, 0.1, rand::random::<u64>() % 3600, "HTTPS server status".to_string(), if https_running { ComponentState::Running } else { ComponentState::Stopped }, None),
                            "packet_processor" => {
                                let proc_metrics = ComponentMetrics::basic(0.5, 512, 10);
                                (packet_processor_running, 0.3, rand::random::<u64>() % 3600, "Packet processor status".to_string(), if packet_processor_running { ComponentState::Running } else { ComponentState::Stopped }, Some(proc_metrics))
                            },
                            _ => (false, 0.0, 0, "Unknown component".to_string(), ComponentState::Unknown, None),
                        };
                        let status_msg = Message::new_status(
                            name, running, uptime, current_load, msg, state, metrics_option,
                        );
                        let _ = status_tx_clone.send(status_msg).await;
                    }
                    ControlCommand::ConfigureComponent { name, config } => {
                        info!("Configuring component {}: {:?}", name, config);
                        let _ = status_tx_clone.send(Message::new_status(
                            name.clone(),
                            true, 0, 0.0,
                            format!("Component {} configured.", name),
                            ComponentState::Running, // Ou um estado de configurando
                            None,
                        )).await;
                    }
                    ControlCommand::Shutdown => {
                        info!("Shutdown command received. Shutting down gracefully...");
                        break; // Sai do loop para encerrar o component_manager
                    }
                    ControlCommand::Unknown => {
                        // Já tratado no nível superior do match
                    }
                }
            } else {
                warn!("Received non-command message in component manager (unexpected): {:?}", message);
            }
        }
    });

    // Monitor de status
    let status_monitor = tokio::spawn(async move {
        while let Some(message) = status_rx.recv().await {
            match message { // Usar match em vez de if let para lidar com todos os Message variants
                Message::Status(status) => {
                    info!(
                        "Component Status - {}: {} (Load: {:.2}, Uptime: {}s) - {} - State: {:?} - Timestamp: {}",
                        status.component_name,
                        if status.is_running { "RUNNING" } else { "STOPPED" },
                        status.current_load,
                        status.uptime_seconds,
                        status.message,
                        status.component_state,
                        status.timestamp
                    );

                    // Serializar e deserializar para testar o sistema de serialização
                    let msg_to_test_serialization = Message::Status(status.clone());

                    match msg_to_test_serialization.serialize() {
                        Ok(serialized) => {
                            match Message::deserialize(&serialized) {
                                Ok(deserialized) => { // Aqui usamos 'deserialized'
                                    info!("Message serialization/deserialization successful (size: {} bytes). Deserialized: {:?}", serialized.len(), deserialized);
                                    // Adicionando um debug log para usar a variável e também verificar se deserializou corretamente.
                                    // Você pode adicionar asserts mais rigorosos aqui se quiser, ex:
                                    // if let Message::Status(s) = deserialized {
                                    //     assert_eq!(s.component_name, status.component_name);
                                    // }
                                }
                                Err(e) => {
                                    error!("Failed to deserialize message for test: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to serialize message for test: {}", e);
                        }
                    }
                }
                Message::Error(error_msg) => {
                    error!(
                        "ERROR - Source: {} - Code: {} - Severity: {:?} - Description: {} - Timestamp: {}",
                        error_msg.source_component,
                        error_msg.error_code,
                        error_msg.severity,
                        error_msg.description,
                        error_msg.timestamp
                    );
                }
                Message::Heartbeat { timestamp } => {
                    debug!("Received Heartbeat at: {}", timestamp);
                }
                Message::Ack { message_id } => {
                    debug!("Received Acknowledge for message ID: {}", message_id);
                }
                Message::Command(_) => { // Comandos não devem vir para o monitor de status
                    warn!("Received Command message in status monitor (unexpected): {:?}", message);
                }
            }
        }
    });

    // 4. Configura os listeners de rede
    let http_listener = TcpListener::bind("0.0.0.0:8080").await
        .context("Failed to bind HTTP listener to 0.0.0.0:8080")?;
    info!("HTTP listener bound to 0.0.0.0:8080");

    let https_listener = TcpListener::bind("0.0.0.0:8443").await
        .context("Failed to bind HTTPS listener to 0.0.0.0:8443")?;
    info!("HTTPS listener bound to 0.0.0.0:8443");

    // Iniciar componentes automaticamente e enviar comandos de teste
    let command_tx_startup_clone = command_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        info!("Sending startup commands to component manager...");
        let _ = command_tx_startup_clone.send(Message::Command(ControlCommand::StartComponent("http_server".to_string()))).await;
        let _ = command_tx_startup_clone.send(Message::Command(ControlCommand::StartComponent("https_server".to_string()))).await;
        let _ = command_tx_startup_clone.send(Message::Command(ControlCommand::StartComponent("packet_processor".to_string()))).await;
        
        // Exemplo de comando de configuração
        let mut settings = std::collections::HashMap::new();
        settings.insert("max_connections".to_string(), "200".to_string());
        settings.insert("buffer_size".to_string(), "8192".to_string());
        let config_cmd = Message::Command(ControlCommand::ConfigureComponent {
            name: "http_server".to_string(),
            config: messages::ComponentConfig { // Usar messages::ComponentConfig
                settings,
                log_level: Some("INFO".to_string()),
                max_resources: Some(messages::ResourceLimits { // Usar messages::ResourceLimits
                    max_cpu: Some(0.7),
                    max_memory_mb: Some(512),
                    max_connections: None, // Exemplo de campo opcional não fornecido
                }),
            },
        });
        let _ = command_tx_startup_clone.send(config_cmd).await;

        // Requisitar status a cada 30 segundos
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            info!("Sending RequestStatus command...");
            let _ = command_tx_startup_clone.send(Message::Command(ControlCommand::RequestStatus)).await;
            
            // Simular o envio de um heartbeat
            let _ = command_tx_startup_clone.send(Message::new_heartbeat()).await;
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
    tokio::spawn(async move {
        info!("Starting simulated RAW packet processing...");
        let mut rng = OsRng;
        let mut packets_processed_total = 0;
        let mut error_count_simulated = 0;
        let mut last_check_time = SystemTime::now();
        let mut packets_since_last_check = 0;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            debug!("Simulating RAW packet reception..."); // Alterado para debug para menos logs verbosos

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
                let _ = status_tx_for_packets.send(error_msg).await;
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
            let metrics = ComponentMetrics::basic(rng.gen::<f32>() * 0.5 + 0.1, rng.gen::<u64>() % 100 + 100, rng.gen::<u32>() % 20)
                .with_total_requests(packets_processed_total)
                .with_error_count(error_count_simulated)
                .with_requests_per_second(rps);
            let status = Message::new_status(
                "packet_processor".to_string(),
                true,
                packets_processed_total as u64 * 3, // Tempo aproximado
                rng.gen::<f32>() * 0.5,
                "Processing packets".to_string(),
                ComponentState::Running,
                Some(metrics), // Passa as métricas para new_status
            );

            let _ = status_tx_for_packets.send(status).await;
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
    }

    Ok(())
}
