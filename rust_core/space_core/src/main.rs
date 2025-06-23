// Declaração dos módulos. Continuam comentados até que sejam preenchidos.
// mod network;
// mod protocols;
// mod ipc;
// mod utils;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration; // Este import é usado pelo tokio::time::sleep
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // <--- ESSAS SÃO AS NOVAS IMPORTAÇÕES CRUCIAIS
use std::env;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

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

    // O runtime Tokio gerencia a multiplexação de I/O (epoll/kqueue/IOCP) automaticamente
    // ao usar operações assíncronas como `listener.accept().await` e `tokio::io::AsyncReadExt`/`AsyncWriteExt`.
    // Isso permite que o servidor lide com milhares de conexões simultâneas de forma eficiente.
    loop {
        let (mut socket, peer_addr) = listener.accept().await?;

        let connection_id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        println!("Nova conexão [ID: {}] de: {}", connection_id, peer_addr);

        // Clona a referência Arc para as conexões ativas para ser movida para a nova tarefa.
        let connections_clone_for_task = Arc::clone(&active_connections);

        // AQUI ESTAVA O PROBLEMA COM `try_clone()`.
        // Para a Tarefa #006, vamos focar apenas no loop de leitura/escrita.
        // O gerenciamento robusto do HashMap com a vida útil do TcpStream será abordado
        // em uma etapa posterior, talvez com canais para notificar fechamentos,
        // ou movendo o socket para dentro do HashMap e usando Arc<Mutex<RefCell<TcpStream>>>
        // ou algo mais complexo.
        // Por enquanto, vamos remover a inserção do `TcpStream` no HashMap aqui
        // e focar em passar a propriedade do `socket` para a tarefa assíncrona.

        // NOTA: Para este exemplo, a `socket` é MOVIDA para a tarefa `tokio::spawn`.
        // Isso significa que o `active_connections` HashMap não terá uma referência direta
        // ao `TcpStream` vivo para esta tarefa específica, e a contagem do HashMap
        // não refletirá a realidade exata de conexões ativas neste momento,
        // pois a remoção depende do `sleep` de exemplo.
        // A precisão da contagem e gerenciamento do HashMap será corrigida quando
        // implementarmos o gerenciamento completo do ciclo de vida das conexões.
        println!("Conexões ativas (temporário, antes de mover para a tarefa): {}", connections_clone_for_task.lock().expect("Failed to lock connections mutex").len());


        tokio::spawn(async move {
            // Buffer para ler os dados da conexão.
            // Um tamanho inicial de 4KB é um bom ponto de partida.
            let mut buf = vec![0u8; 4096];

            println!("[ID: {}] Iniciando loop de leitura/escrita...", connection_id);

            loop {
                // Tenta ler dados do socket.
                // `read()` retorna o número de bytes lidos ou um erro.
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

            // Removendo a conexão do HashMap quando a tarefa termina.
            // Note que isso dependerá de como o HashMap é gerenciado para ser preciso.
            let mut connections_guard = connections_clone_for_task.lock().expect("Failed to lock connections mutex for removal");
            if connections_guard.remove(&connection_id).is_some() {
                // Para que esta remoção funcione corretamente, precisaríamos
                // ter inserido o socket (ou um identificador que permita sua remoção)
                // de forma que ele estivesse lá. No código atual, não estamos
                // inserindo o `TcpStream` real na HashMap de forma que a task o remova.
                // Isso será resolvido em uma futura refatoração do módulo `network`.
                println!("[ID: {}] Conexão de {} removida do gerenciador (simulado).", connection_id, peer_addr);
            } else {
                 // Este `eprintln` é provável de acontecer agora porque o socket real não está no HashMap.
                eprintln!("[ID: {}] Aviso: Conexão de {} não encontrada no gerenciador (esperado nesta fase).", connection_id, peer_addr);
            }
            println!("Conexões ativas restantes (simulado): {}", connections_guard.len());
        });
    }
}