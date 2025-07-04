// Conteúdo para: rs_core/src/network/mod.rs

// Novas importações necessárias
use crate::buffer; // Importa nosso novo módulo (ainda não usado, mas pronto para o futuro)
use bytes::BytesMut; // A estrutura de buffer principal que usaremos
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // AsyncWriteExt é novo
use tokio::net::{TcpListener, TcpStream};

type ConnectionMap = Arc<Mutex<HashMap<usize, SocketAddr>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    // ... (o código da função run_server continua o mesmo até o tokio::spawn) ...
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor ouvindo em http://{}", addr);

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
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


/// Gerencia o ciclo de vida de uma única conexão de cliente.
async fn handle_connection(
    mut socket: TcpStream,
    addr: SocketAddr,
    connections: ConnectionMap,
    id_counter: Arc<AtomicUsize>,
) {
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    connections.lock().unwrap().insert(conn_id, addr);
    println!("[{}] Conexão estabelecida. Total de conexões: {}", conn_id, connections.lock().unwrap().len());

    // Usa BytesMut da crate `bytes`. É um buffer dinâmico e eficiente.
    let mut buffer = BytesMut::with_capacity(1024);

    loop {
        // Tarefa 2: Implementar leitura de dados do socket para o buffer.
        // `read_buf` tenta ler dados para o buffer sem sobrescrever o que já existe.
        match socket.read_buf(&mut buffer).await {
            Ok(0) => {
                println!("[{}] Conexão fechada pelo cliente.", conn_id);
                break;
            }
            Ok(n) => {
                println!("[{}] {} bytes lidos.", conn_id, n);

                // Tarefa 3: Implementar escrita de dados do buffer para o socket.
                // Lógica de "Echo": escreve de volta exatamente o que foi lido.
                if let Err(e) = socket.write_all(&buffer).await {
                    eprintln!("[{}] Erro ao escrever para o socket: {}", conn_id, e);
                    break;
                }
                
                // Limpa o buffer para a próxima leitura.
                buffer.clear();
            }
            Err(e) => {
                eprintln!("[{}] Erro ao ler do socket: {}", conn_id, e);
                break;
            }
        }
    }

    connections.lock().unwrap().remove(&conn_id);
    println!("[{}] Conexão encerrada. Total de conexões: {}", conn_id, connections.lock().unwrap().len());
}