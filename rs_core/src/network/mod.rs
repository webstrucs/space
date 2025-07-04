// Conteúdo para: rs_core/src/network/mod.rs

use bytes::BytesMut;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// Tarefa 2: Enum para representar o estado da conexão na nossa aplicação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Active,
    ShuttingDown,
}

// Tarefa 2: Struct para guardar os dados de uma conexão ativa.
#[derive(Debug, Clone)]
pub struct Connection {
    pub addr: SocketAddr,
    pub state: ConnectionState,
}

// Tarefa 2: Atualizamos nosso mapa para armazenar a struct Connection completa.
type ConnectionMap = Arc<Mutex<HashMap<usize, Connection>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor ouvindo em http://{}", addr);

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        // Tarefa 1: O accept() conclui o handshake TCP.
        match listener.accept().await {
            Ok((socket, addr)) => {
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
    
    // Tarefa 2: Ao aceitar a conexão (pós-handshake), criamos o estado inicial.
    let new_connection = Connection {
        addr,
        state: ConnectionState::Active, // A conexão começa como Ativa.
    };
    connections.lock().unwrap().insert(conn_id, new_connection);
    println!("[{}] Conexão estabelecida. Total de conexões: {}", conn_id, connections.lock().unwrap().len());

    let mut buffer = BytesMut::with_capacity(1024);

    loop {
        match socket.read_buf(&mut buffer).await {
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
    
    // Tarefa 3: Implementar fechamento elegante.
    // Antes de remover do mapa, tentamos um shutdown gracioso do socket.
    if let Err(e) = socket.shutdown().await {
        eprintln!("[{}] Erro durante o shutdown do socket: {}", conn_id, e);
    } else {
        println!("[{}] Socket encerrado elegantemente (FIN enviado).", conn_id);
    }
    
    // Bloco de limpeza final
    connections.lock().unwrap().remove(&conn_id);
    println!("[{}] Conexão removida do mapa. Total de conexões: {}", conn_id, connections.lock().unwrap().len());
}