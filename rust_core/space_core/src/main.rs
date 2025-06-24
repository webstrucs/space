use std::{collections::HashMap, sync::Arc, net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, Semaphore},
    time::sleep,
};
use tracing::{info, warn, error};
use lazy_static::lazy_static;
use governor::{
    Quota,
    RateLimiter,
    state::{InMemoryState, NotKeyed},
    clock::{DefaultClock, Clock},
    Jitter, // Manter importado por enquanto, mesmo que não seja usado diretamente
};
use std::num::NonZeroU32;

// --- Início das adições para Profiling com pprof ---
// #[cfg(feature = "pprof")]
// use pprof::protos::Message; // Descomentado pois não é usado explicitamente
// --- Fim das adições para Profiling com pprof ---

lazy_static! {
    static ref CONNECTIONS: Mutex<HashMap<u32, SocketAddr>> = Mutex::new(HashMap::new());
    static ref NEXT_ID: Mutex<u32> = Mutex::new(1);
    static ref RATE_LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock> = {
        RateLimiter::direct(Quota::per_second(NonZeroU32::new(100_000).unwrap()))
    };
}

// Constante definida corretamente
const MAX_CONCURRENT_CONNECTIONS: usize = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Iniciando servidor Space Core na porta 8080...");
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("Servidor escutando em 127.0.0.1:8080");

    // Correção: Usando o nome correto da constante
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    #[cfg(feature = "pprof")]
    let profiler_guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso", "tokio"])
        .build()
        .unwrap();

    let server_handle = tokio::spawn(async move {
        loop {
            let permit = semaphore.clone().acquire_owned().await
                .expect("Falha ao adquirir permissão do semáforo.");

            match listener.accept().await {
                Ok((socket, addr)) => {
                    let mut connections = CONNECTIONS.lock().await;
                    let id = *NEXT_ID.lock().await;
                    *NEXT_ID.lock().await += 1;
                    connections.insert(id, addr);
                    info!("[ID: {}] Nova conexão de: {}", id, addr);

                    tokio::spawn(async move {
                        handle_client(socket, addr, id).await;
                        drop(permit);
                        let mut connections = CONNECTIONS.lock().await;
                        connections.remove(&id);
                    });
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