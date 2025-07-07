// Conteúdo final, completo e funcional para: rs_core/src/network/mod.rs

use crate::config::Config; // Importa a struct de configuração

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use socket2::{Domain, Protocol, Socket, Type};
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use rustls_pemfile::{certs, private_key};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{error, info, instrument, warn};
use metrics::{counter, gauge};

// --- Módulo Principal e Estruturas de Dados ---
type ConnectionMap = Arc<std::sync::Mutex<HashMap<usize, Connection>>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState { Active, ShuttingDown }

#[derive(Debug, Clone)]
pub struct Connection { pub addr: SocketAddr, pub state: ConnectionState }


// --- Funções Auxiliares ---

fn load_certs_and_key(
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Box<dyn Error>> {
    let certs = certs(&mut BufReader::new(File::open(cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = private_key(&mut BufReader::new(File::open(key_path)?))?
        .ok_or("Nenhuma chave privada encontrada no arquivo.")?;
    Ok((certs, key))
}

fn create_reusable_port_listener(addr: SocketAddr) -> Result<TcpListener, Box<dyn Error>> {
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


// --- Lógica Principal do Servidor e dos Workers ---

pub async fn run_server(config: Config) -> Result<(), Box<dyn Error>> {
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
        info!(worker_id = i, address = %addr, "Worker ouvindo");
        
        let handle = tokio::spawn(async move {
            run_worker(i, listener, acceptor_clone, connections_clone, counter_clone).await;
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
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!(%addr, worker_id, "Nova conexão TCP recebida, iniciando handshake TLS...");
                
                let acceptor_clone = acceptor.clone();
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&id_counter);

                tokio::spawn(async move {
                    match acceptor_clone.accept(socket).await {
                        Ok(tls_stream) => {
                            handle_connection(tls_stream, addr, connections_clone, counter_clone).await;
                        }
                        Err(e) => {
                            warn!(%addr, error = %e, "Falha no handshake TLS");
                        }
                    }
                });
            }
            Err(e) => {
                error!(worker_id, error = %e, "Erro ao aceitar conexão");
            }
        }
    }
}

#[instrument(skip(tls_stream, connections, id_counter), fields(addr = %addr))]
async fn handle_connection(
    mut tls_stream: TlsStream<TcpStream>,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    let new_connection = Connection { addr, state: ConnectionState::Active };
    
    counter!("connections_total").increment(1);
    gauge!("connections_active").increment(1.0);
    info!(conn_id, "Conexão TLS estabelecida");
    
    connections.lock().unwrap().insert(conn_id, new_connection);

    let mut buffer = [0; 1024];
    
    match tls_stream.read(&mut buffer).await {
        Ok(n) if n > 0 => {
            let request_str = String::from_utf8_lossy(&buffer[..n]);
            if let Some(request_line) = request_str.lines().next() {
                info!(conn_id, request_line, "Requisição HTTP recebida.");
                counter!("http_requests_total").increment(1);
            }
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 28\r\n\r\nhello from secure space_server\n";
            if let Err(e) = tls_stream.write_all(response.as_bytes()).await {
                warn!(conn_id, error = %e, "Erro ao escrever resposta HTTP");
            } else {
                counter!("bytes_written_total").increment(response.len() as u64);
            }
        }
        Ok(_) => { 
            info!(conn_id, "Cliente desconectou sem enviar dados.");
        }
        Err(e) => {
            warn!(conn_id, error = %e, "Erro ao ler do stream TLS");
        }
    }

    if let Err(e) = tls_stream.shutdown().await {
        warn!(conn_id, error = %e, "Erro no shutdown do socket");
    }

    connections.lock().unwrap().remove(&conn_id);
    gauge!("connections_active").decrement(1.0);
    info!(conn_id, "Conexão encerrada");
}