// space/rust_core/space_core/src/ipc.rs

use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug, warn};
use anyhow::{Result, Context};
use std::path::Path;
use tokio::sync::{mpsc, broadcast};
use tokio_util::sync::CancellationToken; // Importar CancellationToken

#[allow(unused_imports)]
use crate::messages::{
    Message, ControlCommand, StatusMessage, ErrorMessage, ComponentState, ErrorSeverity,
    message, control_command
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
/// - `status_broadcast_rx`: Um receiver para o canal de broadcast de status.
/// - `command_tx`: Canal para enviar comandos recebidos do Python de volta para o core Rust.
/// - `shutdown_token`: O token de cancelamento para sinalizar o graceful shutdown.
pub async fn start_ipc_server(
    status_broadcast_rx: broadcast::Receiver<Message>,
    command_tx: mpsc::Sender<Message>,
    shutdown_token: CancellationToken, // Recebe o token de shutdown
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
        tokio::select! {
            // Prioriza o sinal de shutdown
            _ = shutdown_token.cancelled() => {
                info!("IPC server received shutdown signal. Stopping accepting new connections.");
                break; // Sai do loop de aceitação
            }
            // Aceitar novas conexões de clientes
            accept_result = listener.accept() => { // Remove .await aqui
                match accept_result.context("Failed to accept IPC connection") {
                    Ok((stream, _addr)) => {
                        info!("Conexão IPC UDS aceita.");

                        let cmd_tx_for_handler = command_tx.clone();
                        let status_rx_for_handler = status_broadcast_rx.resubscribe();
                        let shutdown_token_for_handler = shutdown_token.clone(); // Clona o token para o handler

                        tokio::spawn(async move {
                            if let Err(e) = handle_ipc_client(
                                stream, 
                                cmd_tx_for_handler, 
                                status_rx_for_handler, 
                                shutdown_token_for_handler // Passa o token para o handler do cliente
                            ).await
                                .context("Error handling IPC client connection")
                            {
                                error!("{:?}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        // Este é um erro crítico para o listener, podemos decidir se queremos
                        // tentar novamente ou falhar. Por enquanto, falha.
                        return Err(e);
                    }
                }
            }
        }
    }

    // Opcional: Remover o socket no shutdown limpo
    info!("Removendo socket IPC: {}", IPC_SOCKET_PATH);
    tokio::fs::remove_file(IPC_SOCKET_PATH)
        .await
        .context(format!("Falha ao remover o socket IPC em {} durante o shutdown", IPC_SOCKET_PATH))?;

    info!("Servidor IPC UDS encerrado.");
    Ok(())
}

/// Lida com uma única conexão de cliente IPC UDS.
///
/// Argumentos:
/// - `stream`: O `UnixStream` para a conexão cliente-servidor.
/// - `command_tx`: Sender para enviar comandos recebidos do cliente de volta ao core Rust.
/// - `status_rx`: Receiver para obter mensagens de status do core Rust para enviar ao cliente.
/// - `shutdown_token`: O token de cancelamento para sinalizar o graceful shutdown desta conexão.
async fn handle_ipc_client(
    stream: UnixStream,
    command_tx: mpsc::Sender<Message>,
    mut status_rx: broadcast::Receiver<Message>,
    shutdown_token: CancellationToken, // Recebe o token de shutdown
) -> Result<()> {
    info!("Iniciando handler para cliente IPC.");

    let (mut reader, mut writer) = stream.into_split();
    let mut read_buffer = Vec::new();
    let mut header_buffer = [0u8; 4];

    loop {
        tokio::select! {
            // Prioriza o sinal de shutdown
            _ = shutdown_token.cancelled() => {
                info!("IPC client handler received shutdown signal. Exiting loop.");
                break; // Sai do loop de manipulação do cliente
            }
            // --- Parte de Recebimento de Comandos do Python ---
            read_result = reader.read_exact(&mut header_buffer) => { // Remove .await
                match read_result.context("Failed to read IPC message header") {
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
                        
                        match reader.read_exact(&mut read_buffer).await // Mantém .await aqui
                            .context("Failed to read IPC message data")
                        {
                            Ok(_) => {
                                match Message::deserialize(&read_buffer) {
                                    Ok(message) => {
                                        info!("Comando IPC recebido: {:?}", message);
                                        if let Err(e) = command_tx.send(message).await
                                            .context("Failed to send IPC command to component_manager (channel error)")
                                        {
                                            error!("{:?}", e);
                                            // Se o canal interno fechou, é um erro fatal para esta conexão IPC
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
                                        if let Err(write_err) = send_message_to_client(&mut writer, err_msg).await
                                            .context("Failed to send deserialization error back to IPC client")
                                        {
                                            error!("{:?}", write_err);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("{:?}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        break;
                    }
                }
            }

            // --- Parte de Envio de Status para o Python ---
            status_message = status_rx.recv() => { // Remove .await
                match status_message {
                    Ok(msg) => {
                        if matches!(msg.message_type, Some(message::MessageType::Status(_))) ||
                           matches!(msg.message_type, Some(message::MessageType::Error(_))) {
                            info!("Enviando mensagem de status IPC: {:?}", msg);
                            if let Err(e) = send_message_to_client(&mut writer, msg).await
                                .context("Failed to send IPC status message to client")
                            {
                                error!("{:?}", e);
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

    info!("IPC client handler finished.");
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
