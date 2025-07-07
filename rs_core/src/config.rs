// Conteúdo para: rs_core/src/config.rs

use serde::Deserialize;

// CORREÇÃO: A struct Config precisa ser pública (`pub`) para ser usada fora deste módulo.
#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_cert_path")]
    pub cert_path: String,
    #[serde(default = "default_key_path")]
    pub key_path: String,
}

// Funções que fornecem os valores padrão
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8080 }
fn default_workers() -> usize { num_cpus::get() }
fn default_cert_path() -> String { "certs/cert.pem".to_string() }
fn default_key_path() -> String { "certs/key.pem".to_string() }

impl Config {
    /// Carrega a configuração a partir das variáveis de ambiente.
    pub fn load() -> Result<Self, envy::Error> {
        envy::from_env::<Config>()
    }
}