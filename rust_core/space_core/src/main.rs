// Declaração dos módulos. Continuam comentados até que sejam preenchidos.
// mod network;
// mod protocols;
// mod ipc;
// mod utils;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration; // Usado pelo tokio::time::sleep
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::env;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// NOVAS IMPORTAÇÕES PARA GOVERNOR
use governor::{Quota, RateLimiter};
use governor::state::InMemoryState; // Usaremos InMemoryState para um limite global simples
use std::num::NonZeroU32; // Para definir taxas que não sejam zero

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

    // --- CONFIGURAÇÃO DO RATE LIMITER (TAREFA 7.2) ---
    // Define o limite de taxa para novas conexões:
    // 5 conexões por segundo, com uma capacidade de burst (estouro) de 5.
    // Isso permite que o servidor aceite até 5 conexões de forma instantânea
    // e, após isso, limite a aceitação a 5 novas conexões por segundo.
    let quota = Quota::per_second(NonZeroU32::new(5).expect("quota rate must be non-zero"))
                      .allow_burst(NonZeroU32::new(5).expect("burst size must be non-zero"));

    // Cria o rate limiter global.
    // `RateLimiter::direct(quota)` cria um limiter sem chaves (global).
    // `InMemoryState` é o estado padrão e simples para este tipo de limiter.
    let global_rate_limiter = Arc::new(RateLimiter::direct(quota));
    // --- FIM DA CONFIGURAÇÃO DO RATE LIMITER ---


    // O runtime Tokio gerencia a multiplexação de I/O (epoll/kqueue/IOCP) automaticamente
    // ao usar operações assíncronas como `listener.accept().await` e `tokio::io::AsyncReadExt`/`AsyncWriteExt`.
    // Isso permite que o servidor lide com milhares de conexões simultâneas de forma eficiente.
    loop {
        // Aceita uma nova conexão.
        let (mut socket, peer_addr) = listener.accept().await?;

        // Clona o rate limiter para ser usado na verificação.
        let rate_limiter_for_check = Arc::clone(&global_rate_limiter);

        // --- INÍCIO DA INTEGRAÇÃO DO RATE LIMITER (TAREFA 7.3) ---
        // Verifica se a nova conexão excede o limite.
        // `check()` tenta consumir uma permissão. Se retornar Err, o limite foi atingido.
        if rate_limiter_for_check.check().is_err() {
            println!("Rate limit excedido para nova conexão de: {}. Conexão recusada.", peer_addr);
            // Ao simplesmente retornar ou usar `continue`, o `socket` sai do escopo
            // e é automaticamente fechado pelo Tokio, recusando a conexão.
            continue; // Pula para a próxima iteração do loop para aceitar outra conexão.
        }
        // --- FIM DA INTEGRAÇÃO DO RATE LIMITER ---

        let connection_id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        println!("Nova conexão [ID: {}] de: {}", connection_id, peer_addr);

        // Clona a referência Arc para o gerenciador de conexões ativas para ser movida para a nova tarefa.
        let connections_clone_for_task = Arc::clone(&active_connections);

        // NOTA: Para a Tarefa #006, simplificamos o gerenciamento
        // do HashMap. O `TcpStream` é MOVIDO para a tarefa `tokio::spawn`, o que significa
        // que o `active_connections` HashMap não mantém uma referência direta ao `TcpStream` vivo aqui.
        // A precisão do HashMap será abordada em uma futura refatoração do módulo `network`.
        println!("Conexões ativas (temporário, antes de mover para a tarefa): {}", connections_clone_for_task.lock().expect("Failed to lock connections mutex").len());

        // A tarefa assíncrona para lidar com a conexão (echo server).
        tokio::spawn(async move {
            // Buffer para ler os dados da conexão.
            let mut buf = vec![0u8; 4096];

            println!("[ID: {}] Iniciando loop de leitura/escrita (echo server)...", connection_id);

            loop {
                // Tenta ler dados do socket.
                let bytes_read = match socket.read(&mut buf).await {
                    Ok(0) => { // Conexão fechada pelo cliente.
                        println!("[ID: {}] Conexão fechada por {}", connection_id, peer_addr);
                        break; // Sai do loop para terminar a tarefa.
                    },
                    Ok(n) => n, // Bytes lidos com sucesso.
                    Err(e) => { // Erro na leitura.
                        eprintln!("[ID: {}] Erro ao ler do socket {}: {}", connection_id, peer_addr, e);
                        break; // Sai do loop em caso de erro.
                    }
                };

                // Escreve os bytes lidos de volta no mesmo socket (comportamento de echo).
                if let Err(e) = socket.write_all(&buf[..bytes_read]).await {
                    eprintln!("[ID: {}] Erro ao escrever no socket {}: {}", connection_id, peer_addr, e);
                    break; // Sai do loop em caso de erro na escrita.
                }

                println!("[ID: {}] Ecoado {} bytes para {}", connection_id, bytes_read, peer_addr);
            }

            // Lógica de remoção simulada do HashMap.
            // Esta lógica ainda precisa ser aprimorada para o gerenciamento preciso do ciclo de vida.
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