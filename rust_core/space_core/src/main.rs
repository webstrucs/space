// Declaração dos módulos. Continuam comentados até que sejam preenchidos.
// mod network;
// mod protocols;
// mod ipc;
// mod utils;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration}; // Para o idle timeout
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::env;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration as StdDuration; // Para o TCP Keep-Alive, para evitar conflito com tokio::time::Duration

// IMPORTAÇÕES PARA GOVERNOR
use governor::{Quota, RateLimiter};
use governor::state::InMemoryState; // Usaremos InMemoryState para um limite global simples
use std::num::NonZeroU32; // Para definir taxas que não sejam zero

// IMPORTAÇÕES PARA SOCKET2
use socket2::{SockRef, TcpKeepalive};

// Alias para a nossa estrutura de gerenciamento de conexões.
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

    let active_connections: Connections = Arc::new(Mutex::new(HashMap::new()));

    // --- CONFIGURAÇÃO DO RATE LIMITER (AJUSTADO PARA 100.000 CPS) ---
    // Define o limite de taxa para novas conexões:
    // 100.000 conexões por segundo, com uma capacidade de burst (estouro) de 100.000.
    // Isso permite ao servidor aceitar um alto volume de novas conexões.
    let quota = Quota::per_second(NonZeroU32::new(100_000).expect("quota rate must be non-zero"))
                          .allow_burst(NonZeroU32::new(100_000).expect("burst size must be non-zero"));

    let global_rate_limiter = Arc::new(RateLimiter::direct(quota));
    // --- FIM DA CONFIGURAÇÃO DO RATE LIMITER ---

    loop {
        // Aceita uma nova conexão.
        let (socket, peer_addr) = listener.accept().await?;

        // Clona o rate limiter para ser usado na verificação.
        let rate_limiter_for_check = Arc::clone(&global_rate_limiter);

        // --- INTEGRAÇÃO DO RATE LIMITER ---
        if rate_limiter_for_check.check().is_err() {
            println!("Rate limit excedido para nova conexão de: {}. Conexão recusada.", peer_addr);
            continue; // Pula para a próxima iteração do loop.
        }
        // --- FIM DA INTEGRAÇÃO DO RATE LIMITER ---

        let connection_id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        println!("Nova conexão [ID: {}] de: {}", connection_id, peer_addr);

        // --- CONFIGURAÇÃO DO TCP KEEP-ALIVE ---
        // A maneira correta de obter um SockRef de um TcpStream do Tokio
        // para aplicar opções de socket é via `SockRef::from(TcpStream)`.
        let sock_ref = SockRef::from(&socket);

        // Configura os parâmetros do TCP Keep-Alive
        let keepalive_config = TcpKeepalive::new()
            .with_time(StdDuration::from_secs(60))    // Tempo de inatividade antes de enviar o primeiro probe
            .with_interval(StdDuration::from_secs(10)) // Intervalo entre probes
            .with_retries(3); // Número de probes antes de considerar a conexão morta

        if let Err(e) = sock_ref.set_tcp_keepalive(&keepalive_config) {
            eprintln!("[ID: {}] Erro ao configurar TCP Keep-Alive para {}: {}", connection_id, peer_addr, e);
        } else {
            println!("[ID: {}] TCP Keep-Alive configurado (idle=60s, int=10s, probes=3) para {}", connection_id, peer_addr);
        }
        // --- FIM DA CONFIGURAÇÃO DO TCP KEEP-ALIVE ---

        let connections_clone_for_task = Arc::clone(&active_connections);
        println!("Conexões ativas (temporário, antes de mover para a tarefa): {}", connections_clone_for_task.lock().expect("Failed to lock connections mutex").len());

        // A tarefa assíncrona para lidar com a conexão (echo server).
        tokio::spawn(async move {
            let mut socket = socket; // Torna o socket mutável dentro da task.

            // --- ANÁLISE DE BUFFER OVERFLOW ---
            // Buffer para ler os dados da conexão.
            // Em Rust, `vec!` garante que o buffer tem um tamanho fixo de 4096 bytes.
            // A função `socket.read()` nunca escreverá mais bytes do que a capacidade do buffer,
            // prevenindo buffer overflows. Se a entrada exceder 4096 bytes, será lida em pedaços.
            let mut buf = vec![0u8; 4096];
            // --- FIM DA ANÁLISE DE BUFFER OVERFLOW ---

            println!("[ID: {}] Iniciando loop de leitura/escrita (echo server) com idle timeout...", connection_id);

            loop {
                // --- INTEGRAÇÃO DO IDLE TIMEOUT ---
                let read_op = socket.read(&mut buf);
                let read_result = timeout(Duration::from_secs(30), read_op).await;

                let bytes_read = match read_result {
                    Ok(Ok(n)) => {
                        if n == 0 {
                            println!("[ID: {}] Conexão fechada por {}", connection_id, peer_addr);
                            break;
                        }
                        n
                    },
                    Ok(Err(e)) => {
                        eprintln!("[ID: {}] Erro ao ler do socket {}: {}", connection_id, peer_addr, e);
                        break;
                    },
                    Err(_) => { // O timeout de inatividade foi atingido.
                        println!("[ID: {}] Timeout de inatividade de 30 segundos atingido para {}. Encerrando conexão.", connection_id, peer_addr);
                        break;
                    }
                };

                // Escreve os bytes lidos de volta no mesmo socket (comportamento de echo).
                if let Err(e) = socket.write_all(&buf[..bytes_read]).await {
                    eprintln!("[ID: {}] Erro ao escrever no socket {}: {}", connection_id, peer_addr, e);
                    break;
                }

                println!("[ID: {}] Ecoado {} bytes para {}", connection_id, bytes_read, peer_addr);
            }

            // Lógica de remoção simulada do HashMap.
            let mut connections_guard = connections_clone_for_task.lock().expect("Failed to lock connections mutex for removal");
            if connections_guard.remove(&connection_id).is_some() {
                println!("[ID: {}] Conexão de {} removida do gerenciador (simulado).", connection_id, peer_addr);
            } else {
                eprintln!("[ID: {}] Aviso: Conexão de {} não encontrada no gerenciador (esperado nesta fase).", connection_id, peer_addr);
            }
            println!("Conexões ativas restantes (simulado): {}", connections_guard.len());
        });
    }
}