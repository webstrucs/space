// Conteúdo para: rs_core/src/error.rs

use thiserror::Error;
use tokio::task::JoinError;

// Usamos o 'rustls' que é re-exportado pelo 'tokio_rustls' para consistência de tipos.
use tokio_rustls::rustls;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de Parsing de Endereço: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Erro de TLS: {0}")]
    Tls(#[from] rustls::Error),
    
    #[error("Erro ao carregar certificados ou chaves: {0}")]
    CertLoad(String),

    #[error("Erro de configuração das variáveis de ambiente: {0}")]
    Config(#[from] envy::Error),

    #[error("Erro em tarefa concorrente: {0}")]
    Join(#[from] JoinError),
}

pub type Result<T> = std::result::Result<T, AppError>;