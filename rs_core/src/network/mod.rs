// Conteúdo final e funcional para: rs_core/src/network/mod.rs

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::ipc::protocol::LowLevelMessage;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use socket2::{Domain, Protocol, Socket, Type};
use tokio::sync::broadcast;
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use rustls_pemfile::{certs, private_key};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UnixStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{error, info, instrument, warn};
use metrics::{counter, gauge};

type ConnectionMap = Arc<std::sync::Mutex<HashMap<usize, Connection>>>;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState { Active, ShuttingDown }
#[derive(Debug, Clone)]
pub struct Connection { pub addr: SocketAddr, pub state: ConnectionState }

fn load_certs_and_key(
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let certs = certs(&mut std::io::BufReader::new(std::fs::File::open(cert_path)?))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let key = private_key(&mut std::io::BufReader::new(std::fs::File::open(key_path)?))?
        .ok_or_else(|| AppError::CertLoad("Nenhuma chave privada encontrada no arquivo.".to_string()))?;
    Ok((certs, key))
}

fn create_reusable_port_listener(addr: SocketAddr) -> Result<TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.bind(&addr.into())?;
    socket.listen(1024)?;
    let std_listener: std::net::TcpListener = socket.into();
    std_listener.set_nonblocking(true)?;
    let tokio_listener = TcpListener::from_std(std_listener)?;
    Ok(tokio_listener)
}

pub async fn run_server(config: Config, shutdown_tx: broadcast::Sender<()>) -> Result<()> {
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let num_workers = config.workers;
    info!(workers = num_workers, "Iniciando o servidor Space...");

    let (certs, key) = load_certs_and_key(&config.cert_path, &config.key_path)?;
    
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    
    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
    
    let mut worker_handles = Vec::with_capacity(num_workers);

    for i in 0..num_workers {
        let connections_clone = Arc::clone(&connections);
        let counter_clone = Arc::clone(&connection_id_counter);
        let acceptor_clone = acceptor.clone();
        let listener = create_reusable_port_listener(addr)?;
        let shutdown_rx = shutdown_tx.subscribe();
        
        info!(worker_id = i, address = %addr, "Worker ouvindo");
        
        let handle = tokio::spawn(async move {
            run_worker(i, listener, acceptor_clone, shutdown_rx, connections_clone, counter_clone).await;
        });
        worker_handles.push(handle);
    }

    for handle in worker_handles {
        handle.await?;
    }

    Ok(())
}

async fn run_worker(
    worker_id: usize,
    listener: TcpListener,
    acceptor: TlsAcceptor,
    mut shutdown_rx: broadcast::Receiver<()>,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.recv() => {
                info!(worker_id, "Worker recebendo sinal de shutdown.");
                break;
            }
            result = listener.accept() => {
                if let Ok((socket, addr)) = result {
                    info!(%addr, worker_id, "Nova conexão TCP recebida");
                    
                    let acceptor_clone = acceptor.clone();
                    let connections_clone = Arc::clone(&connections);
                    let counter_clone = Arc::clone(&id_counter);

                    tokio::spawn(async move {
                        if let Ok(tls_stream) = acceptor_clone.accept(socket).await {
                            handle_connection(tls_stream, addr, connections_clone, counter_clone).await;
                        } else {
                            warn!(%addr, "Falha no handshake TLS");
                        }
                    });
                } else if let Err(e) = result {
                    error!(worker_id, error = %e, "Erro ao aceitar conexão");
                }
            }
        }
    }
}

#[instrument(skip_all, fields(conn_id, addr))]
async fn handle_connection(
    mut tls_stream: TlsStream<TcpStream>,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst) as u64;
    tracing::Span::current().record("conn_id", conn_id);
    tracing::Span::current().record("addr", &tracing::field::display(addr));

    info!("Conexão TLS estabelecida");
    let new_connection = Connection { addr, state: ConnectionState::Active };
    connections.lock().unwrap().insert(conn_id as usize, new_connection);
    
    counter!("connections_total").increment(1);
    gauge!("connections_active").increment(1.0);
    
    let mut buffer = [0; 1024];
    
    if let Ok(n) = tls_stream.read(&mut buffer).await {
        if n > 0 {
             let request_data = buffer[..n].to_vec();
            info!(bytes_read = n, "Requisição recebida do cliente.");

            let message = LowLevelMessage::Data { conn_id, data: request_data };
            if let Ok(serialized_message) = bincode::serialize(&message) {
                const IPC_SOCKET_PATH: &str = "/tmp/space_server.sock";
                if let Ok(mut ipc_stream) = UnixStream::connect(IPC_SOCKET_PATH).await {
                    if let Err(e) = ipc_stream.write_all(&serialized_message).await {
                        warn!(error = %e, "Falha ao enviar dados via IPC.");
                    } else {
                        info!(bytes_sent = serialized_message.len(), "Dados enviados via IPC.");
                    }
                }
            }
            let response = b"HTTP/1.1 200 OK\r\n\r\nIPC message sent\n";
            if let Err(e) = tls_stream.write_all(response).await {
                warn!(error = %e, "Erro ao escrever resposta HTTP");
            }
        }
    }

    if let Err(e) = tls_stream.shutdown().await {
        warn!(error = %e, "Erro no shutdown do socket");
    }

    connections.lock().unwrap().remove(&(conn_id as usize));
    gauge!("connections_active").decrement(1.0);
    info!("Conexão encerrada");
}