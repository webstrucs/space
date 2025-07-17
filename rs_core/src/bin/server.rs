// Conteúdo final e funcional para: rs_core/src/bin/server.rs

use rs_core::config::Config;
use rs_core::error::Result;
use rs_core::network::{run_https_server, run_http_redirect_server};
use axum::{routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tracing::{error, info};
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
    let subscriber = FmtSubscriber::builder().with_max_level(tracing::Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Falha ao configurar o subscriber de tracing");

    let (shutdown_tx, _) = broadcast::channel(1);

    let shutdown_future = async {
        tokio::signal::ctrl_c().await.expect("Falha ao ouvir Ctrl+C");
        info!("Sinal de Ctrl+C recebido, enviando sinal de shutdown...");
        if shutdown_tx.send(()).is_err() {
            error!("Nenhum receptor ativo para o sinal de shutdown.");
        }
    };

    let builder = PrometheusBuilder::new();
    let handle = builder.install_recorder().expect("Falha ao instalar o recorder de métricas");
    tokio::spawn(start_metrics_server(handle));

    let config = Config::load()?;
    info!(config = ?&config, "Configuração carregada");
    
    // Prepara os futuros para os dois servidores.
    let https_server_future = run_https_server(config, shutdown_tx.clone());
    let http_redirect_future = run_http_redirect_server(shutdown_tx.subscribe());

    // Executa os dois servidores e o listener de shutdown concorrentemente.
    tokio::select! {
        biased;
        res = https_server_future => {
            if let Err(e) = res { error!(error = %e, "Servidor HTTPS falhou fatalmente."); }
        }
        res = http_redirect_future => {
            if let Err(e) = res { error!(error = %e, "Servidor de Redirecionamento HTTP falhou fatalmente."); }
        }
        _ = shutdown_future => {
            info!("Sinal de shutdown tratado, processo principal será encerrado.");
        }
    }

    Ok(())
}