// Conteúdo para: rs_core/src/network/mod.rs

use std::collections::HashMap;
use std::error::Error;
use std::net::{IpAddr, SocketAddr}; // IpAddr foi adicionado
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant}; // Instant foi adicionado
use tokio::net::{TcpListener, TcpStream};
use socket2::SockRef;

// Constantes para o Rate Limiting
const RATE_LIMIT_WINDOW_SECS: u64 = 60; // Janela de tempo de 1 minuto
const RATE_LIMIT_MAX_CONN: usize = 10;   // Máximo de 10 conexões por IP por minuto

// ... (structs e types continuam os mesmos) ...
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState { Active, ShuttingDown }
#[derive(Debug, Clone)]
pub struct Connection { pub addr: SocketAddr, pub state: ConnectionState }
type ConnectionMap = Arc<Mutex<HashMap<usize, Connection>>>;
// Novo tipo para o nosso mapa de rate limiting
type RateLimitMap = Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Servidor ouvindo em http://{}", addr);

    let connection_id_counter = Arc::new(AtomicUsize::new(0));
    let connections: ConnectionMap = Arc::new(Mutex::new(HashMap::new()));
    // Inicializa a estrutura de dados para o rate limiter
    let rate_limiter: RateLimitMap = Arc::new(Mutex::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                // --- INÍCIO DA LÓGICA DE RATE LIMITING ---
                let client_ip = addr.ip();
                let mut limiter = rate_limiter.lock().unwrap();

                // Obtém a lista de timestamps para este IP, ou cria uma nova se não existir
                let timestamps = limiter.entry(client_ip).or_insert_with(Vec::new);

                // Remove timestamps que estão fora da janela de tempo
                let now = Instant::now();
                timestamps.retain(|&t| now.duration_since(t).as_secs() < RATE_LIMIT_WINDOW_SECS);

                // Verifica se o limite foi excedido
                if timestamps.len() >= RATE_LIMIT_MAX_CONN {
                    eprintln!("[SEGURANÇA] Rate limit excedido para o IP: {}. Conexão recusada.", client_ip);
                    continue; // Pula para a próxima iteração do loop, descartando a conexão
                }

                // Adiciona o timestamp da conexão atual
                timestamps.push(now);
                // --- FIM DA LÓGICA DE RATE LIMITING ---
                
                // Drop o lock antes de prosseguir para não segurá-lo desnecessariamente
                drop(limiter);

                // Configuração do Keep-Alive (código anterior)
                let socket_ref = SockRef::from(&socket);
                let keepalive = socket2::TcpKeepalive::new().with_time(Duration::from_secs(60));
                if let Err(e) = socket_ref.set_tcp_keepalive(&keepalive) {
                    eprintln!("[{}] Erro ao configurar Keep-Alive: {}", addr, e);
                }
                
                println!("Nova conexão de: {}", addr);
                let connections_clone = Arc::clone(&connections);
                let counter_clone = Arc::clone(&connection_id_counter);
                let _rate_limiter_clone = Arc::clone(&rate_limiter);

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
// ... (a função handle_connection continua a mesma) ...
async fn handle_connection(
    _socket: TcpStream,
    _addr: SocketAddr,
    _connections: ConnectionMap,
    _id_counter: Arc<AtomicUsize>,
) { 
    // ...
}