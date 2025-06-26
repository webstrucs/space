use std::{collections::HashMap, sync::Arc, net::SocketAddr};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, Semaphore, mpsc},
    time::sleep,
};
use tracing::{info, warn, error};
use lazy_static::lazy_static;
use governor::{
    Quota,
    RateLimiter,
    state::{InMemoryState, NotKeyed},
    clock::{DefaultClock, Clock},
};
use std::num::NonZeroU32;
use socket2::{Socket, SockAddr, Protocol, Type, Domain}; // Importações corretas

// Variáveis globais para rastreamento de conexões e IDs
lazy_static! {
    static ref CONNECTIONS: Mutex<HashMap<u32, SocketAddr>> = Mutex::new(HashMap::new());
    static ref NEXT_ID: Mutex<u32> = Mutex::new(1);
    static ref RATE_LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock> = {
        RateLimiter::direct(Quota::per_second(NonZeroU32::new(100_000).unwrap()))
    };
}

const MAX_CONCURRENT_CONNECTIONS: usize = 1000;
const NUM_WORKER_TASKS: usize = 4;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Iniciando servidor Space Core na porta 8080...");

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    
    let socket = Socket::new(
        Domain::for_address(addr), // Obtém o domínio correto (IPv4 ou IPv6) do SocketAddr
        Type::STREAM,            // **CORREÇÃO AQUI:** Usar Type::STREAM (constante)
        Some(Protocol::TCP),     // **CORREÇÃO AQUI:** Usar Protocol::TCP (constante)
    )?;
    
    socket.set_reuse_address(true)?;
    #[cfg(target_os = "linux")]
    socket.set_reuse_port(true)?;

    socket.bind(&SockAddr::from(addr))?;
    socket.listen(1024)?;

    let listener = TcpListener::from_std(socket.into())?;

    info!("Servidor escutando em 127.0.0.1:8080 com SO_REUSEPORT");

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    // --- Início do código para iniciar o Profiler ---
    #[cfg(feature = "pprof")]
    let profiler_guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso", "tokio"])
        .build()
        .unwrap();
    // --- Fim do código para iniciar o Profiler ---

    // --- Balanceamento de Carga: Canais e Workers ---
    let mut tx_senders: Vec<mpsc::Sender<TcpStream>> = Vec::with_capacity(NUM_WORKER_TASKS);

    for i in 0..NUM_WORKER_TASKS {
        let (tx, rx) = mpsc::channel::<TcpStream>(100);
        tx_senders.push(tx);

        let worker_semaphore = semaphore.clone();

        tokio::spawn(async move {
            info!("Worker {} iniciado e esperando por conexões...", i);
            worker_task(rx, worker_semaphore).await;
        });
    }

    let mut current_worker_idx = 0;

    let server_handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let connections = CONNECTIONS.lock().await;
                    let id = *NEXT_ID.lock().await;
                    *NEXT_ID.lock().await += 1;
                    info!("[ID: {}] Conexão recebida de: {} (enviando para worker {})", id, addr, current_worker_idx);

                    let tx = &tx_senders[current_worker_idx];
                    if let Err(e) = tx.send(socket).await {
                        error!("[ID: {}] Falha ao enviar conexão para o worker {}: {}", id, current_worker_idx, e);
                    }

                    current_worker_idx = (current_worker_idx + 1) % NUM_WORKER_TASKS;
                }
                Err(e) => error!("Erro ao aceitar conexão: {}", e),
            }
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Sinal de interrupção recebido (Ctrl+C). Encerrando servidor...");
        },
        _ = server_handle => {
            warn!("Servidor encerrou inesperadamente.");
        }
    }

    #[cfg(feature = "pprof")]
    {
        info!("Gerando Flame Graph...");
        if let Ok(report) = profiler_guard.report().build() {
            let mut file = std::fs::File::create("flamegraph.svg")
                .expect("Não foi possível criar flamegraph.svg");
            report.flamegraph(&mut file)
                .expect("Não foi possível escrever os dados do flamegraph");
            info!("Flame Graph gerado em flamegraph.svg");
        } else {
            warn!("Não foi possível gerar o Flame Graph.");
        }
    }

    Ok(())
}

async fn worker_task(mut receiver: mpsc::Receiver<TcpStream>, semaphore: Arc<Semaphore>) {
    while let Some(stream) = receiver.recv().await {
        let permit = semaphore.clone().acquire_owned().await
            .expect("Falha ao adquirir permissão do semáforo no worker.");
        
        let peer_addr = stream.peer_addr().ok();
        let id = *NEXT_ID.lock().await;

        handle_client(stream, peer_addr.unwrap_or_else(|| "0.0.0.0:0".parse().unwrap()), id).await;
        drop(permit);
    }
    info!("Worker task encerrada.");
}

async fn handle_client(mut stream: TcpStream, addr: SocketAddr, id: u32) {
    let mut buffer = vec![0; 1024];
    loop {
        loop {
            match RATE_LIMITER.check() {
                Ok(_) => break,
                Err(not_ready_until) => {
                    let wait_time = not_ready_until.wait_time_from(DefaultClock::default().now());
                    sleep(wait_time).await;
                }
            }
        }

        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("[ID: {}] Conexão fechada por {}", id, addr);
                break;
            }
            Ok(n) => {
                if let Err(e) = stream.write_all(&buffer[0..n]).await {
                    error!("[ID: {}] Erro ao ecoar dados para {}: {}", id, addr, e);
                    break;
                }
                info!("[ID: {}] Ecoado {} bytes para {}", id, n, addr);
            }
            Err(e) => {
                error!("[ID: {}] Erro de leitura da conexão {}: {}", id, addr, e);
                break;
            }
        }
    }
}