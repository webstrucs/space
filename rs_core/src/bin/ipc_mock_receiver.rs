// Conteúdo para: rs_core/src/bin/ipc_mock_receiver.rs

use rs_core::ipc::protocol::LowLevelMessage;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use std::fs;

const IPC_SOCKET_PATH: &str = "/tmp/space_server.sock";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[MOCK PYTHON] Iniciando o receptor IPC...");

    // Garante que o arquivo do socket não exista de uma execução anterior.
    if fs::metadata(IPC_SOCKET_PATH).is_ok() {
        println!("[MOCK PYTHON] Removendo arquivo de socket antigo: {}", IPC_SOCKET_PATH);
        fs::remove_file(IPC_SOCKET_PATH)?;
    }

    // 1. Cria um "ouvinte" de Socket de Domínio Unix.
    let listener = UnixListener::bind(IPC_SOCKET_PATH)?;
    println!("[MOCK PYTHON] Ouvindo no socket IPC: {}", IPC_SOCKET_PATH);

    // 2. Aguarda por uma conexão do nosso servidor Rust principal.
    match listener.accept().await {
        Ok((mut stream, _addr)) => {
            println!("[MOCK PYTHON] Conexão IPC recebida!");
            let mut buffer = Vec::new();

            // 3. Lê todos os dados enviados pelo servidor principal.
            stream.read_to_end(&mut buffer).await?;
            println!("[MOCK PYTHON] {} bytes recebidos.", buffer.len());

            // 4. Tenta desserializar os bytes para a nossa struct LowLevelMessage.
            let message: LowLevelMessage = bincode::deserialize(&buffer)?;

            println!("[MOCK PYTHON] Mensagem desserializada com sucesso:");
            println!("{:#?}", message);
        }
        Err(e) => {
            eprintln!("[MOCK PYTHON] Erro ao aceitar conexão IPC: {}", e);
        }
    }

    Ok(())
}