// Conteúdo completo para: rs_core/src/ipc/protocol.rs

use serde::{Deserialize, Serialize};

/// Mensagens enviadas do núcleo Rust (baixo nível) para a lógica Python (alto nível).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum LowLevelMessage {
    /// Notifica que uma nova conexão foi estabelecida.
    NewConnection {
        conn_id: u64,
        remote_addr: String,
    },
    /// Envia os dados brutos de uma requisição para serem processados.
    Data {
        conn_id: u64,
        // CORREÇÃO: Adicionamos o endereço do cliente a esta mensagem.
        remote_addr: String,
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