// Conteúdo final e corrigido para: rs_core/src/network/mod.rs

use socket2::{Socket, Domain, Type, Protocol};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt; // AsyncReadExt agora será usada
use tokio::net::{TcpListener, TcpStream};

const TCP_KEEPALIVE_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState { Active, ShuttingDown }
#[derive(Debug, Clone)]
pub struct Connection { pub addr: SocketAddr, pub state: ConnectionState }
type ConnectionMap = Arc<Mutex<HashMap<usize, Connection>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let num_workers = num_cpus::get();
    println!("[INFO] Utilizando {} workers (um por núcleo de CPU).", num_workers);

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));
    
    let mut worker_handles = Vec::with_capacity(num_workers);

    for i in 0..num_workers {
        let connections_clone = Arc::clone(&connections);
        let counter_clone = Arc::clone(&connection_id_counter);
        
        let listener = create_reusable_port_listener(addr)?;
        println!("[Worker {}] Ouvindo em http://{}", i, addr);
        
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
                // Lógica de Keep-Alive reintegrada aqui, dentro do worker.
                let socket_ref = socket2::SockRef::from(&socket);
                let keepalive = socket2::TcpKeepalive::new().with_time(Duration::from_secs(TCP_KEEPALIVE_SECS));
                if let Err(e) = socket_ref.set_tcp_keepalive(&keepalive) {
                    eprintln!("[Worker {}] Erro ao configurar Keep-Alive para {}: {}", worker_id, addr, e);
                }

                println!("[Worker {}] Nova conexão de: {}", worker_id, addr);
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&id_counter);

                tokio::spawn(async move {
                    handle_connection(socket, addr, connections_clone, counter_clone).await;
                });
            }
            Err(e) => {
                eprintln!("[Worker {}] Erro ao aceitar conexão: {}", worker_id, e);
            }
        }
    }
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

// A função handle_connection completa, igual à da Issue #008

async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    let new_connection = Connection { addr, state: ConnectionState::Active };
    connections.lock().unwrap().insert(conn_id, new_connection);

    // NÃO vamos ler nada do socket.
    // Em vez disso, vamos escrever uma resposta imediatamente.
    if let Err(e) = socket.write_all(b"hello from space_server\n").await {
        eprintln!("[Conn {}] Erro ao escrever 'hello': {}", conn_id, e);
    }
    
    // E então fechamos a conexão imediatamente.
    if let Err(e) = socket.shutdown().await {
        eprintln!("[Conn {}] Erro no shutdown: {}", conn_id, e);
    }

    connections.lock().unwrap().remove(&conn_id);
}