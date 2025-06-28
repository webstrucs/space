// space/rust_core/space_core/src/main.rs

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use std::sync::Arc;
use anyhow::Context;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use rand::rngs::OsRng;
use rand::Rng;

mod handlers;
mod metrics;
mod tls;
mod packets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configuração do logger (tracing)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting space_core server...");

    // CORREÇÃO: Inicializar o provedor criptográfico antes de usar TLS
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install default crypto provider"))?;
    info!("Crypto provider initialized.");

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

    // 3. Configura os listeners de rede
    let http_listener = TcpListener::bind("0.0.0.0:8080").await
        .context("Failed to bind HTTP listener to 0.0.0.0:8080")?;
    info!("HTTP listener bound to 0.0.0.0:8080");

    let https_listener = TcpListener::bind("0.0.0.0:8443").await
        .context("Failed to bind HTTPS listener to 0.0.0.0:8443")?;
    info!("HTTPS listener bound to 0.0.0.0:8443");

    // 4. Spawna tarefas para lidar com conexões HTTP
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

    // 5. Spawna tarefas para lidar com conexões HTTPS
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
    tokio::spawn(async move {
        info!("Starting simulated RAW packet processing...");
        let mut rng = OsRng;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            info!("Simulating RAW packet reception...");

            crate::metrics::RAW_PACKETS_TOTAL.increment(1);

            if rng.gen_bool(0.7) {
                crate::metrics::RAW_IPV4_PACKETS_TOTAL.increment(1);
                packets::process_packet_data(&[0x45, 0x00, 0x00, 0x34, 0x00, 0x01, 0x00, 0x00, 0x40, 0x06, 0x7c, 0xb0, 0x7f, 0x00, 0x00, 0x01, 0x7f, 0x00, 0x00, 0x01]);
            } else {
                crate::metrics::RAW_IPV6_PACKETS_TOTAL.increment(1);
                packets::process_packet_data(&[0x60, 0x00, 0x00, 0x00, 0x00, 0x14, 0x06, 0x40, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);
            }
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
    }

    Ok(())
}