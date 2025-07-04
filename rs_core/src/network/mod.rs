// Conteúdo para: rs_core/src/network/mod.rs

use bytes::BytesMut;
use socket2::{Socket, Domain, Type}; // Importações para o Keep-Alive
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration; // Importação para definir durações de tempo
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout; // Importação da função de timeout

// Tarefa 1: Constante para o timeout de inatividade (ex: 10 segundos)
const IDLE_TIMEOUT_SECS: u64 = 10;
// Tarefa 2: Constante para o tempo do TCP Keep-Alive (ex: 60 segundos)
const TCP_KEEPALIVE_SECS: u64 = 60;

// ... (structs ConnectionState, Connection e type ConnectionMap continuam iguais) ...
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Active,
    ShuttingDown,
}
#[derive(Debug, Clone)]
pub struct Connection {
    pub addr: SocketAddr,
    pub state: ConnectionState,
}
type ConnectionMap = Arc<Mutex<HashMap<usize, Connection>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor ouvindo em http://{}", addr);

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                // Tarefa 2: Configurar o TCP Keep-Alive no socket recém-aceito
                let socket_ref = socket2::SockRef::from(&socket);
                let keepalive = socket2::TcpKeepalive::new().with_time(Duration::from_secs(TCP_KEEPALIVE_SECS));
                if let Err(e) = socket_ref.set_tcp_keepalive(&keepalive) {
                    eprintln!("[{}] Erro ao configurar Keep-Alive: {}", addr, e);
                } else {
                    println!("[{}] TCP Keep-Alive configurado.", addr);
                }

                println!("Nova conexão de: {}", addr);
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&connection_id_counter);

                tokio::spawn(async move {
                    handle_connection(socket, addr, connections_clone, counter_clone).await;
                });
            }
            Err(e) => {
                eprintln!("Erro ao aceitar conexão: {}", e);
            }
        }
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    let new_connection = Connection { addr, state: ConnectionState::Active };
    connections.lock().unwrap().insert(conn_id, new_connection);
    println!("[{}] Conexão estabelecida. Total de conexões: {}", conn_id, connections.lock().unwrap().len());

    let mut buffer = BytesMut::with_capacity(1024);
    let idle_duration = Duration::from_secs(IDLE_TIMEOUT_SECS);

    loop {
        // Tarefa 1: Envelopa a operação de leitura com um timeout.
        let read_operation = socket.read_buf(&mut buffer);
        
        match timeout(idle_duration, read_operation).await {
            // Caso 1: Timeout ocorreu
            Err(_) => {
                println!("[{}] Timeout de inatividade. Encerrando conexão.", conn_id);
                break;
            }
            // Caso 2: Operação de leitura completou (com sucesso ou erro)
            Ok(result) => match result {
                Ok(0) => {
                    println!("[{}] Conexão fechada pelo cliente.", conn_id);
                    break;
                }
                Ok(n) => {
                    println!("[{}] {} bytes lidos.", conn_id, n);
                    if let Err(e) = socket.write_all(&buffer).await {
                        eprintln!("[{}] Erro ao escrever para o socket: {}", conn_id, e);
                        break;
                    }
                    buffer.clear();
                }
                Err(e) => {
                    eprintln!("[{}] Erro ao ler do socket: {}", conn_id, e);
                    break;
                }
            }
        }
    }

    if let Err(e) = socket.shutdown().await {
        eprintln!("[{}] Erro durante o shutdown do socket: {}", conn_id, e);
    } else {
        println!("[{}] Socket encerrado elegantemente (FIN enviado).", conn_id);
    }
    
    connections.lock().unwrap().remove(&conn_id);
    println!("[{}] Conexão removida do mapa. Total de conexões: {}", conn_id, connections.lock().unwrap().len());
}