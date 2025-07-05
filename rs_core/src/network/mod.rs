// Conteúdo final e corrigido para: rs_core/src/network/mod.rs

use metrics::{counter, gauge};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, instrument, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState { Active, ShuttingDown }
#[derive(Debug, Clone)]
pub struct Connection { pub addr: SocketAddr, pub state: ConnectionState }
type ConnectionMap = Arc<Mutex<HashMap<usize, Connection>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let num_workers = num_cpus::get();
    info!(workers = num_workers, "Iniciando o servidor Space...");

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));
    
    let mut worker_handles = Vec::with_capacity(num_workers);

    for i in 0..num_workers {
        let connections_clone = Arc::clone(&connections);
        let counter_clone = Arc::clone(&connection_id_counter);
        
        let listener = create_reusable_port_listener(addr)?;
        info!(worker_id = i, address = %addr, "Worker ouvindo");
        
        let handle = tokio::spawn(async move {
            run_worker(i, listener, connections_clone, counter_clone).await;
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
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!(%addr, worker_id, "Nova conexão recebida");
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&id_counter);

                tokio::spawn(async move {
                    handle_connection(socket, addr, connections_clone, counter_clone).await;
                });
            }
            Err(e) => {
                error!(worker_id, error = %e, "Erro ao aceitar conexão");
            }
        }
    }
}

// CORREÇÃO: Corpo da função restaurado
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

#[instrument(skip(socket, connections, id_counter), fields(addr = %addr))]
async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    let new_connection = Connection { addr, state: ConnectionState::Active };
    
    counter!("connections_total").increment(1);
    gauge!("connections_active").increment(1.0);
    info!(conn_id, "Conexão estabelecida");
    
    connections.lock().unwrap().insert(conn_id, new_connection);

    let hello_msg = b"hello from space_server\n";
    if let Err(e) = socket.write_all(hello_msg).await {
        warn!(conn_id, error = %e, "Erro ao escrever 'hello'");
    } else {
        counter!("bytes_written_total").increment(hello_msg.len() as u64);
    }
    
    if let Err(e) = socket.shutdown().await {
        warn!(conn_id, error = %e, "Erro no shutdown do socket");
    }

    connections.lock().unwrap().remove(&conn_id);
    gauge!("connections_active").decrement(1.0);
    info!(conn_id, "Conexão encerrada");
}