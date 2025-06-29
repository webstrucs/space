// space/rust_core/space_core/src/ipc.rs

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug, warn};
use anyhow::{Result, Context};
use std::path::Path;
use tokio::sync::{mpsc, broadcast};

// Permitir imports não utilizados, pois esses tipos podem ser acessados via `Message` enum
// e são parte da API de mensagens (principalmente para clareza ou testes futuros).
#[allow(unused_imports)]
use crate::messages::{
    Message, ControlCommand, StatusMessage, ErrorMessage, ComponentState, ErrorSeverity,
    message, control_command // Precisamos importar os módulos internos gerados para acessar os enums `oneof`
};

/// O caminho para o socket de domínio Unix.
const IPC_SOCKET_PATH: &str = "/tmp/space_core_ipc.sock";

/// Inicia o servidor Unix Domain Socket (UDS) para comunicação IPC.
///
/// Este servidor escutará por conexões no `IPC_SOCKET_PATH`. Quando um cliente
/// (como a camada Python) se conectar, ele estabelecerá um canal de comunicação
/// bidirecional para enviar mensagens de status e receber comandos.
///
/// Argumentos:
/// - `status_broadcast_rx`: Um receiver para o canal de broadcast de status. Cada cliente IPC
///   terá sua própria cópia clonada deste receiver.
/// - `command_tx`: Canal para enviar comandos recebidos do Python de volta para o core Rust.
pub async fn start_ipc_server(
    status_broadcast_rx: broadcast::Receiver<Message>,
    command_tx: mpsc::Sender<Message>,
) -> Result<()> {
    // 1. Limpar socket antigo se existir
    if Path::new(IPC_SOCKET_PATH).exists() {
        info!("Removendo socket IPC antigo: {}", IPC_SOCKET_PATH);
        tokio::fs::remove_file(IPC_SOCKET_PATH)
            .await
            .context(format!("Falha ao remover o socket existente em {}", IPC_SOCKET_PATH))?;
    }

    // 2. Criar e bindar o Unix Listener
    let listener = UnixListener::bind(IPC_SOCKET_PATH)
        .context(format!("Falha ao bindar o socket IPC em {}", IPC_SOCKET_PATH))?;

    info!("Servidor IPC UDS escutando em: {}", IPC_SOCKET_PATH);

    loop {
        // 3. Aceitar novas conexões de clientes
        match listener.accept().await {
            Ok((stream, _addr)) => {
                info!("Conexão IPC UDS aceita.");

                let cmd_tx_for_handler = command_tx.clone();
                let status_rx_for_handler = status_broadcast_rx.resubscribe(); 

                tokio::spawn(async move {
                    if let Err(e) = handle_ipc_client(stream, cmd_tx_for_handler, status_rx_for_handler).await {
                        error!("Erro ao lidar com cliente IPC: {:?}", e);
                    }
                });
            }
            Err(e) => {
                error!("Erro ao aceitar conexão IPC: {:?}", e);
                return Err(e).context("Erro crítico no listener IPC");
            }
        }
    }
}

/// Lida com uma única conexão de cliente IPC UDS.
///
/// Esta função estabelece dois loops principais: um para receber comandos do cliente
/// e outro para enviar mensagens de status do Rust para o cliente.
///
/// Argumentos:
/// - `stream`: O `UnixStream` para a conexão cliente-servidor.
/// - `command_tx`: Sender para enviar comandos recebidos do cliente de volta ao core Rust.
/// - `status_rx`: Receiver para obter mensagens de status do core Rust para enviar ao cliente.
async fn handle_ipc_client(
    stream: UnixStream,
    command_tx: mpsc::Sender<Message>,
    mut status_rx: broadcast::Receiver<Message>,
) -> Result<()> {
    info!("Iniciando handler para cliente IPC.");

    let (mut reader, mut writer) = stream.into_split();
    let mut read_buffer = Vec::new();
    let mut header_buffer = [0u8; 4];

    loop {
        tokio::select! {
            // --- Parte de Recebimento de Comandos do Python ---
            read_result = reader.read_exact(&mut header_buffer) => {
                match read_result {
                    Ok(0) => {
                        info!("Cliente IPC desconectado.");
                        break;
                    }
                    Ok(_) => {
                        let message_size = u32::from_le_bytes(header_buffer) as usize;
                        debug!("Recebido cabeçalho de mensagem IPC com tamanho: {} bytes", message_size);

                        if message_size == 0 {
                            debug!("Mensagem IPC vazia recebida, ignorando.");
                            continue;
                        }

                        read_buffer.resize(message_size, 0u8);
                        
                        match reader.read_exact(&mut read_buffer).await {
                            Ok(_) => {
                                match Message::deserialize(&read_buffer) {
                                    Ok(message) => {
                                        info!("Comando IPC recebido: {:?}", message);
                                        if let Err(e) = command_tx.send(message).await {
                                            error!("Falha ao enviar comando para o component_manager (canal fechado ou erro): {:?}", e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Falha ao desserializar comando IPC: {:?}", e);
                                        let err_msg = Message::new_error(
                                            500,
                                            format!("Erro de desserialização de comando IPC: {}", e),
                                            "ipc_server".to_string(),
                                            ErrorSeverity::Error,
                                        );
                                        if let Err(write_err) = send_message_to_client(&mut writer, err_msg).await {
                                            error!("Falha ao enviar erro de desserialização de volta ao cliente: {:?}", write_err);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Falha ao ler dados da mensagem IPC: {:?}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Falha ao ler cabeçalho da mensagem IPC: {:?}", e);
                        break;
                    }
                }
            }

            // --- Parte de Envio de Status para o Python ---
            status_message = status_rx.recv() => {
                match status_message {
                    Ok(msg) => {
                        if matches!(msg.message_type, Some(message::MessageType::Status(_))) ||
                           matches!(msg.message_type, Some(message::MessageType::Error(_))) {
                            info!("Enviando mensagem de status IPC: {:?}", msg);
                            if let Err(e) = send_message_to_client(&mut writer, msg).await {
                                error!("Falha ao enviar mensagem de status IPC: {:?}", e);
                                break;
                            }
                        } else {
                            debug!("Mensagem não-status/erro não enviada ao cliente IPC: {:?}", msg);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Receiver IPC atrasado, pulou {} mensagens de status. Considerar aumentar o tamanho do buffer do broadcast channel.", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Canal de status de broadcast fechado, encerrando handler IPC.");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Envia uma mensagem serializada para o cliente IPC.
///
/// A mensagem é prefixada com seu tamanho (u32 em little-endian) para que o receptor
/// saiba quantos bytes ler.
async fn send_message_to_client(writer: &mut tokio::net::unix::OwnedWriteHalf, message: Message) -> Result<()> {
    let serialized_message = message.serialize()?;
    let message_size = serialized_message.len() as u32;

    writer.write_all(&message_size.to_le_bytes()).await.context("Falha ao escrever tamanho da mensagem IPC")?;
    writer.write_all(&serialized_message).await.context("Falha ao escrever dados da mensagem IPC")?;
    writer.flush().await.context("Falha ao flushar stream IPC")?;

    Ok(())
}
