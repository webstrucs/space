// Declaração dos módulos. Continuam comentados até que sejam preenchidos.
// mod network;
// mod protocols;
// mod ipc;
// mod utils;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration;
use std::env;
use std::sync::{Arc, Mutex}; // Para compartilhar estado entre threads assíncronas
use std::collections::HashMap; // Para armazenar as conexões ativas
use std::sync::atomic::{AtomicUsize, Ordering}; // Para IDs de conexão únicos e seguros

// Alias para a nossa estrutura de gerenciamento de conexões.
// Arc: Permite múltiplos "donos" para os dados, clonando a referência.
// Mutex: Garante que apenas uma tarefa acesse a HashMap por vez para evitar condições de corrida.
// HashMap: Armazena as conexões (ID da conexão -> TcpStream).
type Connections = Arc<Mutex<HashMap<usize, TcpStream>>>;

// Contador atômico para gerar IDs únicos para cada conexão.
static CONNECTION_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&addr).await?;

    println!("Space Core Server: Escutando em {}", addr);
    println!("Aguardando conexões...");

    // Cria a estrutura compartilhada para gerenciar as conexões ativas.
    let active_connections: Connections = Arc::new(Mutex::new(HashMap::new()));

    // O runtime Tokio gerencia a multiplexação de I/O (epoll/kqueue/IOCP) automaticamente
    // ao usar operações assíncronas como `listener.accept().await` e `tokio::io::AsyncReadExt`/`AsyncWriteExt`.
    // Isso permite que o servidor lide com milhares de conexões simultâneas de forma eficiente.
    loop {
        let (mut socket, peer_addr) = listener.accept().await?;

        // Gera um ID único para a nova conexão.
        let connection_id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        println!("Nova conexão [ID: {}] de: {}", connection_id, peer_addr);

        // Clona a referência Arc para as conexões ativas para ser movida para a nova tarefa.
        let connections_clone = Arc::clone(&active_connections);

        // Bloqueia o Mutex para adicionar a nova conexão.
        // .expect() é usado aqui para simplificar; em produção, trataria o erro de "poisoning".
        connections_clone.lock().expect("Failed to lock connections mutex").insert(connection_id, socket);

        println!("Conexões ativas: {}", connections_clone.lock().expect("Failed to lock connections mutex").len());

        // A tarefa assíncrona para lidar com esta conexão.
        tokio::spawn(async move {
            // Aqui é onde a lógica de leitura/escrita do socket irá morar.
            // Por enquanto, apenas simula um trabalho e depois remove a conexão.
            println!("[ID: {}] Processando conexão...", connection_id);
            tokio::time::sleep(Duration::from_secs(5)).await; // Simula um trabalho mais longo

            // Quando a tarefa termina (ou a conexão é fechada), removemos do HashMap.
            let mut connections_guard = connections_clone.lock().expect("Failed to lock connections mutex for removal");
            if connections_guard.remove(&connection_id).is_some() {
                println!("[ID: {}] Conexão de {} processada e removida.", connection_id, peer_addr);
            } else {
                eprintln!("[ID: {}] Erro: Conexão de {} não encontrada ao tentar remover.", connection_id, peer_addr);
            }

            println!("Conexões ativas restantes: {}", connections_guard.len());
        });
    }
}