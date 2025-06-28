// space/rust_core/space_core/src/config.rs

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub listen_address: String,
    pub http_port: u16,
    pub https_port: u16,
    pub metrics_port: u16,
    // Adicione outras configurações conforme necessário
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        info!("Carregando configurações a partir de variáveis de ambiente...");
        let config = envy::from_env::<AppConfig>()
            .context("Falha ao carregar configurações de variáveis de ambiente. Verifique se todas as variáveis necessárias (LISTEN_ADDRESS, HTTP_PORT, HTTPS_PORT, METRICS_PORT) estão definidas.")?;
        Ok(config)
    }
}