// Conteúdo para: rs_core/src/bin/server.rs

use rs_core::config::Config;
use rs_core::error::Result;
use axum::{routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

async fn start_metrics_server(handle: PrometheusHandle) {
    let app = Router::new().route("/metrics", get(move || async move { handle.render() }));
    let addr = SocketAddr::from(([127, 0, 0, 1], 9090));
    info!("Servidor de métricas ouvindo em http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Falha ao configurar o subscriber de tracing");

    let (shutdown_tx, _) = broadcast::channel(1);

    // O shutdown_future vai usar o shutdown_tx original.
    let shutdown_future = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Falha ao ouvir o sinal de Ctrl+C");
        info!("Sinal de Ctrl+C recebido, enviando sinal de shutdown...");
        shutdown_tx.send(()).expect("Falha ao enviar sinal de shutdown");
    };

    let builder = PrometheusBuilder::new();
    let handle = builder.install_recorder().expect("Falha ao instalar recorder");
    tokio::spawn(start_metrics_server(handle));

    let config = Config::load()?;
    tracing::info!(config = ?&config, "Configuração carregada");
    
    // CORREÇÃO: Passamos um clone do sender para o servidor principal.
    // Agora, tanto o `shutdown_future` quanto o `server_future` têm sua própria "alça" válida.
    let server_future = rs_core::network::run_server(config, shutdown_tx.clone());

    tokio::select! {
        res = server_future => {
            if let Err(e) = res {
                tracing::error!(error = %e, "Erro fatal no servidor principal.");
            }
        }
        _ = shutdown_future => {
            info!("Sinal de shutdown tratado, processo principal será encerrado.");
        }
    }

    Ok(())
}