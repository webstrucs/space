// Conteúdo para: rs_core/src/ipc/protocol.rs

use serde::{Deserialize, Serialize};

// Usamos enums para definir um "protocolo" de mensagens.
// O #[derive(...)] gera o código para serializar/desserializar automaticamente.

/// Mensagens enviadas do núcleo Rust (baixo nível) para a lógica Python (alto nível).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum LowLevelMessage {
    /// Notifica que uma nova conexão foi estabelecida.
    NewConnection {
        conn_id: u64,
        remote_addr: String, // Usamos String para simplicidade na serialização entre linguagens.
    },
    /// Envia os dados brutos de uma requisição para serem processados.
    Data {
        conn_id: u64,
        data: Vec<u8>,
    },
    /// Informa que o cliente fechou a conexão.
    ConnectionClosed {
        conn_id: u64,
    },
}

/// Mensagens enviadas da lógica Python (alto nível) para o núcleo Rust (baixo nível).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum HighLevelMessage {
    /// Envia os dados de resposta que devem ser escritos de volta para o cliente.
    ResponseData {
        conn_id: u64,
        data: Vec<u8>,
    },
    /// Solicita que o núcleo Rust encerre uma conexão específica.
    CloseConnection {
        conn_id: u64,
    },
}