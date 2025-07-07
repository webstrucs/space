// Conteúdo final e unificado para: rs_core/src/bin/server.rs

use rs_core::config::Config;
use axum::{routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use tracing_subscriber::FmtSubscriber;

// Função para rodar o servidor web de métricas
async fn start_metrics_server(handle: PrometheusHandle) {
    let app = Router::new().route("/metrics", get(move || async move { handle.render() }));
    let addr = SocketAddr::from(([127, 0, 0, 1], 9090));
    tracing::info!("Servidor de métricas ouvindo em http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() {
    // 1. Configura o Logging (da Issue #012)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Falha ao configurar o subscriber de tracing");

    // 2. Configura o Coletor de Métricas (da Issue #012)
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("Falha ao instalar o recorder de métricas");

    // 3. Carrega a Configuração (da Issue #014)
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!(error = %e, "Erro fatal ao carregar a configuração. Desligando.");
            std::process::exit(1);
        }
    };
    
    tracing::info!(config = ?&config, "Configuração carregada");

    // 4. Inicia o servidor de métricas em uma tarefa separada (lógica da Issue #012 reintegrada)
    tokio::spawn(start_metrics_server(handle));

    // 5. Inicia o servidor principal com a configuração carregada
    if let Err(e) = rs_core::network::run_server(config).await {
        tracing::error!(error = %e, "Erro fatal no servidor principal. Desligando.");
        std::process::exit(1);
    }
}