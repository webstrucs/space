// Conteúdo para: rs_core/src/network/mod.rs

use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

// Tarefa 4: Estrutura de dados para gerenciar conexões.
// Usamos um type alias para facilitar a leitura.
// Arc -> Permite compartilhamento seguro entre threads.
// Mutex -> Garante que apenas uma thread acesse o HashMap por vez.
type ConnectionMap = Arc<Mutex<HashMap<usize, SocketAddr>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor ouvindo em http://{}", addr);

    // Tarefa 4: Inicializa nossa estrutura de dados.
    // Contador atômico para gerar IDs únicos para cada conexão.
    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    // O mapa de conexões em si.
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                println!("Nova conexão de: {}", addr);

                // Tarefa 3: Multiplexação com tokio::spawn.
                // Clonamos os ponteiros para as estruturas compartilhadas.
                // Isso apenas aumenta a contagem de referências, não clona os dados.
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&connection_id_counter);

                // Inicia uma nova tarefa verde (green thread) para lidar com a conexão.
                // O `move` transfere a propriedade das variáveis clonadas para a nova tarefa.
                tokio::spawn(async move {
                    // Passamos os recursos para a função que gerencia a conexão.
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
    // Gera um ID único para esta nova conexão.
    // Ordering::SeqCst garante a consistência da memória entre threads.
    let conn_id = id_counter.fetch_add(1, Ordering::SeqCst);
    
    // Adiciona a conexão ao nosso mapa compartilhado.
    // .lock() adquire o "cadeado" do mutex, e .unwrap() assume que o lock não falhará.
    connections.lock().unwrap().insert(conn_id, addr);
    println!("[{}] Conexão estabelecida. Total de conexões: {}", conn_id, connections.lock().unwrap().len());

    // Buffer para ler os dados do socket.
    let mut buf = [0; 1024];

    // Loop para ler dados do cliente.
    loop {
        match socket.read(&mut buf).await {
            // Se read() retornar Ok(0), o cliente fechou a conexão.
            Ok(0) => {
                println!("[{}] Conexão fechada pelo cliente.", conn_id);
                break;
            }
            Ok(n) => {
                let data = String::from_utf8_lossy(&buf[0..n]);
                println!("[{}] Recebido: {}", conn_id, data);
                // Futuramente, aqui passaremos os dados para a camada de alto nível (Python).
            }
            Err(e) => {
                eprintln!("[{}] Erro ao ler do socket: {}", conn_id, e);
                break;
            }
        }
    }

    // Bloco de limpeza: remove a conexão do mapa quando o loop termina.
    connections.lock().unwrap().remove(&conn_id);
    println!("[{}] Conexão encerrada. Total de conexões: {}", conn_id, connections.lock().unwrap().len());
}