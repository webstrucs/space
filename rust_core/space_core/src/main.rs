// space/rust_core/space_core/src/main.rs

use std::{
    sync::Arc,
    net::SocketAddr,
    fs,
    io::{self, BufReader},
    num::NonZeroU32,
};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Mutex, Semaphore, mpsc},
    time::sleep, // <--- **CORREÇÃO CRÍTICA**: `sleep` é de `tokio::time`
};
use tracing::{info, warn, error, debug};
use lazy_static::lazy_static;
use governor::{
    Quota,
    RateLimiter,
    state::{InMemoryState, NotKeyed},
    clock::{DefaultClock, Clock},
};
use socket2::{Socket, SockAddr, Protocol, Type, Domain};

// Importações para métricas
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics::{increment_gauge, decrement_gauge, counter};

// Importações para TLS e HTTP parsing
use tokio_rustls::{rustls, TlsAcceptor};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig as TlsServerConfig; // Renomeado para evitar conflito com nossa AppConfig
use rustls_pemfile::{self, Item};
use httparse;

// --- INÍCIO DAS MUDANÇAS PARA A ISSUE #014 ---
use serde::Deserialize; // Importa a trait Deserialize do Serde
use anyhow::{Context, Result}; // Importa Context e Result do Anyhow para tratamento de erros

// Estrutura que representa as configurações do aplicativo, carregadas do ambiente
#[derive(Debug, Deserialize)] // Permite que a struct seja impressa para depuração e deserializada pelo envy
pub struct AppConfig {
    // Porta para o servidor HTTPS. Se HTTPS_PORT não for definida no ambiente, usa 8443.
    #[serde(default = "default_https_port")]
    pub https_port: u16,
    // Porta para o servidor HTTP. Se HTTP_PORT não for definida no ambiente, usa 8080.
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    // Porta para o exportador de métricas Prometheus. Se METRICS_PORT não for definida, usa 9000.
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

// Funções para fornecer valores padrão para a AppConfig
fn default_https_port() -> u16 { 8443 }
fn default_http_port() -> u16 { 8080 }
fn default_metrics_port() -> u16 { 9000 }
// --- FIM DAS MUDANÇAS PARA A ISSUE #014 ---


lazy_static! {
    static ref NEXT_ID: Mutex<u32> = Mutex::new(1);
    static ref RATE_LIMITER: RateLimiter<NotKeyed, InMemoryState, DefaultClock> = {
        RateLimiter::direct(Quota::per_second(NonZeroU32::new(100_000).unwrap()))
    };
}

const MAX_CONCURRENT_CONNECTIONS: usize = 1000;
const NUM_WORKER_TASKS: usize = 4;

#[tokio::main]
// Altere a assinatura para usar anyhow::Result
async fn main() -> Result<()> { // Agora retorna um Result do anyhow
    tracing_subscriber::fmt::init();

    info!("Iniciando servidor Space Core...");

    // Tenta carregar as configurações das variáveis de ambiente
    let config = match envy::from_env::<AppConfig>() {
        Ok(conf) => {
            info!("Configuração carregada com sucesso do ambiente: {:?}", conf);
            conf
        }
        Err(err) => {
            // Se houver um erro (ex: variável de ambiente mal formatada), loga o erro e falha.
            error!("Erro ao carregar configurações do ambiente: {}. Verifique o formato das variáveis (ex: HTTPS_PORT=8443, HTTP_PORT=8080, METRICS_PORT=9000).", err);
            return Err(anyhow::anyhow!("Falha ao carregar configurações de ambiente: {}", err));
        }
    };


    // Configuração do Exportador de Métricas Prometheus
    let builder = PrometheusBuilder::new();
    // Use a porta da `config`
    match builder.with_http_listener(([0, 0, 0, 0], config.metrics_port)).install() {
        Ok(_) => info!("Exportador de métricas Prometheus iniciado em 0.0.0.0:{}", config.metrics_port),
        Err(e) => error!("Falha ao iniciar exportador Prometheus: {}", e),
    }

    // Carregar certificado e chave privada para TLS
    // Use .context() do anyhow para adicionar contexto aos erros
    let certs = load_certs("cert.pem").context("Falha ao carregar certificados TLS")?;
    let key = load_keys("key.pem").context("Falha ao carregar chave privada TLS")?;

    let tls_server_config = TlsServerConfig::builder() // Note a renomeação de ServerConfig para TlsServerConfig
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let acceptor = TlsAcceptor::from(Arc::new(tls_server_config));
    let acceptor_clone = acceptor.clone();

    // Listener para HTTPS
    // Use a porta da `config`
    let https_addr: SocketAddr = format!("0.0.0.0:{}", config.https_port).parse()?;
    let https_socket = Socket::new(Domain::for_address(https_addr), Type::STREAM, Some(Protocol::TCP))?;
    https_socket.set_reuse_address(true)?;
    #[cfg(target_os = "linux")]
    https_socket.set_reuse_port(true)?;
    https_socket.bind(&SockAddr::from(https_addr))?;
    https_socket.listen(1024)?;
    https_socket.set_nonblocking(true)?;
    let https_listener = TcpListener::from_std(https_socket.into())?;
    info!("Servidor HTTPS escutando em 0.0.0.0:{} com SO_REUSEPORT", config.https_port);

    // Listener para HTTP
    // Use a porta da `config`
    let http_addr: SocketAddr = format!("127.0.0.1:{}", config.http_port).parse()?;
    let http_socket = Socket::new(Domain::for_address(http_addr), Type::STREAM, Some(Protocol::TCP))?;
    http_socket.set_reuse_address(true)?;
    #[cfg(target_os = "linux")]
    http_socket.set_reuse_port(true)?;
    http_socket.bind(&SockAddr::from(http_addr))?;
    http_socket.listen(1024)?;
    http_socket.set_nonblocking(true)?;
    let http_listener = TcpListener::from_std(http_socket.into())?;
    info!("Servidor HTTP escutando em 127.0.0.1:{} com SO_REUSEPORT", config.http_port);

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    #[cfg(feature = "pprof")]
    let profiler_guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso", "tokio"])
        .build()
        .unwrap();

    let mut tx_senders: Vec<mpsc::Sender<(TcpStream, Option<TlsAcceptor>)>> = Vec::with_capacity(NUM_WORKER_TASKS);

    for i in 0..NUM_WORKER_TASKS {
        let (tx, rx) = mpsc::channel::<(TcpStream, Option<TlsAcceptor>)>(100);
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
            tokio::select! {
                Ok((socket, addr)) = http_listener.accept() => {
                    let id = *NEXT_ID.lock().await;
                    *NEXT_ID.lock().await += 1;
                    counter!("space_core_total_connections_received", 1);
                    increment_gauge!("space_core_active_connections", 1.0);

                    info!("[ID: {}] Conexão HTTP recebida de: {} (enviando para worker {})", id, addr, current_worker_idx);

                    let tx = &tx_senders[current_worker_idx];
                    if let Err(e) = tx.send((socket, None)).await {
                        error!("[ID: {}] Falha ao enviar conexão HTTP para o worker {}: {}", id, current_worker_idx, e);
                        decrement_gauge!("space_core_active_connections", 1.0);
                    }
                    current_worker_idx = (current_worker_idx + 1) % NUM_WORKER_TASKS;
                },
                Ok((socket, addr)) = https_listener.accept() => {
                    let id = *NEXT_ID.lock().await;
                    *NEXT_ID.lock().await += 1;
                    counter!("space_core_total_connections_received", 1);
                    increment_gauge!("space_core_active_connections", 1.0);

                    info!("[ID: {}] Conexão HTTPS recebida de: {} (enviando para worker {})", id, addr, current_worker_idx);

                    let tx = &tx_senders[current_worker_idx];
                    if let Err(e) = tx.send((socket, Some(acceptor_clone.clone()))).await {
                        error!("[ID: {}] Falha ao enviar conexão HTTPS para o worker {}: {}", id, current_worker_idx, e);
                        decrement_gauge!("space_core_active_connections", 1.0);
                    }
                    current_worker_idx = (current_worker_idx + 1) % NUM_WORKER_TASKS;
                }
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

// Função auxiliar para carregar certificados
fn load_certs(path: &str) -> io::Result<Vec<CertificateDer<'static>>> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    let items: Vec<Item> = rustls_pemfile::read_all(&mut reader)
        .collect::<Result<Vec<Item>, _>>()?;

    let certs: Vec<CertificateDer> = items
        .into_iter()
        .filter_map(|item| {
            if let Item::X509Certificate(cert) = item {
                Some(cert)
            } else {
                None
            }
        })
        .collect();

    if certs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No certificates found in PEM file"));
    }

    Ok(certs)
}

// Função auxiliar para carregar chaves privadas
fn load_keys(path: &str) -> io::Result<PrivateKeyDer<'static>> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);

    let items: Vec<Item> = rustls_pemfile::read_all(&mut reader)
        .collect::<Result<Vec<Item>, _>>()?;

    for item in items {
        match item {
            Item::Pkcs1Key(key) => return Ok(PrivateKeyDer::Pkcs1(key)),
            Item::Pkcs8Key(key) => return Ok(PrivateKeyDer::Pkcs8(key)),
            Item::Sec1Key(key) => return Ok(PrivateKeyDer::Sec1(key)),
            _ => continue,
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidInput, "No private key found in PEM file"))
}

async fn worker_task(mut receiver: mpsc::Receiver<(TcpStream, Option<TlsAcceptor>)>, semaphore: Arc<Semaphore>) {
    while let Some((stream, tls_acceptor_opt)) = receiver.recv().await {
        let permit = semaphore.clone().acquire_owned().await
            .expect("Falha ao adquirir permissão do semáforo no worker.");

        let peer_addr = stream.peer_addr().ok();
        let id = *NEXT_ID.lock().await;

        debug!("[ID: {}] Worker iniciando processamento da conexão de {}", id, peer_addr.unwrap_or_else(|| "UNKNOWN".parse().unwrap()));

        match tls_acceptor_opt {
            Some(acceptor) => {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        info!("[ID: {}] Handshake TLS bem-sucedido para {}", id, peer_addr.unwrap_or_else(|| "UNKNOWN".parse().unwrap()));
                        let _ = handle_tls_client(tls_stream, peer_addr.unwrap_or_else(|| "0.0.0.0:0".parse().unwrap()), id).await;
                    }
                    Err(e) => {
                        error!("[ID: {}] Falha no handshake TLS para {}: {}", id, peer_addr.unwrap_or_else(|| "UNKNOWN".parse().unwrap()), e);
                        decrement_gauge!("space_core_active_connections", 1.0);
                    }
                }
            },
            None => {
                let _ = handle_http_client(stream, peer_addr.unwrap_or_else(|| "0.0.0.0:0".parse().unwrap()), id).await;
            }
        }

        debug!("[ID: {}] Worker finalizou processamento da conexão de {}", id, peer_addr.unwrap_or_else(|| "UNKNOWN".parse().unwrap()));

        decrement_gauge!("space_core_active_connections", 1.0);
        drop(permit);
    }
    info!("Worker task encerrada.");
}

async fn handle_tls_client(mut tls_stream: tokio_rustls::server::TlsStream<TcpStream>, addr: SocketAddr, id: u32) -> io::Result<()> {
    let mut buffer = vec![0; 4096]; // Increased buffer size

    loop {
        // Rate limiting
        loop {
            match RATE_LIMITER.check() {
                Ok(_) => break,
                Err(not_ready_until) => {
                    let wait_time = not_ready_until.wait_time_from(DefaultClock::default().now());
                    debug!("[ID: {}] Rate limit atingido. Esperando por {}ms (TLS).", id, wait_time.as_millis());
                    sleep(wait_time).await;
                }
            }
        }

        match tls_stream.read(&mut buffer).await {
            Ok(0) => {
                info!("[ID: {}] Conexão TLS fechada por {}", id, addr);
                counter!("space_core_total_connections_closed", 1);
                break;
            }
            Ok(n) => {
                counter!("space_core_bytes_read", n as u64);
                debug!("[ID: {}] Lido {} bytes de {} (TLS)", id, n, addr);

                // Parse HTTP request over TLS
                let mut headers = [httparse::EMPTY_HEADER; 16];
                let mut req = httparse::Request::new(&mut headers);

                match req.parse(&buffer[0..n]) {
                    Ok(httparse::Status::Complete(_bytes_consumed)) => {
                        info!("[ID: {}] Requisição HTTPS recebida: Método='{}' Caminho='{}' HTTP/{:?}",
                            id,
                            req.method.unwrap_or(""),
                            req.path.unwrap_or(""),
                            req.version.unwrap_or(0),
                        );
                        debug!("[ID: {}] HTTPS Headers: {:?}", id, req.headers);

                        let response: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 25\r\nContent-Type: text/plain\r\n\r\nHello from HTTPS server!";
                        if let Err(e) = tls_stream.write_all(response).await {
                             error!("[ID: {}] Erro ao enviar resposta HTTPS para {}: {}", id, addr, e);
                             counter!("space_core_write_errors_total", 1);
                             break;
                        }
                        counter!("space_core_bytes_written", response.len() as u64);
                        counter!("space_core_requests_processed_total", 1);
                        info!("[ID: {}] Processada e respondida HTTPS para {}", id, addr);
                        break; // Close connection after response
                    },
                    Ok(httparse::Status::Partial) => {
                        debug!("[ID: {}] Requisição HTTPS parcial. Precisa ler mais bytes.", id);
                        continue; // Continue reading more data
                    },
                    Err(e) => {
                        warn!("[ID: {}] Erro ao fazer parsing HTTPS: {}", id, e);
                        let error_response: &[u8] = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                        if let Err(e) = tls_stream.write_all(error_response).await {
                             error!("[ID: {}] Erro ao enviar erro 400 HTTPS para {}: {}", id, addr, e);
                        }
                        counter!("space_core_write_errors_total", 1);
                        break;
                    },
                }
            }
            Err(e) => {
                error!("[ID: {}] Erro de leitura da conexão TLS {}: {}", id, addr, e);
                counter!("space_core_read_errors_total", 1);
                break;
            }
        }
    }
    Ok(())
}

async fn handle_http_client(mut stream: TcpStream, addr: SocketAddr, id: u32) -> io::Result<()> {
    let mut buffer = vec![0; 4096]; // Increased buffer size

    loop {
        // Rate limiting
        loop {
            match RATE_LIMITER.check() {
                Ok(_) => break,
                Err(not_ready_until) => {
                    let wait_time = not_ready_until.wait_time_from(DefaultClock::default().now());
                    debug!("[ID: {}] Rate limit atingido. Esperando por {}ms (HTTP).", id, wait_time.as_millis());
                    sleep(wait_time).await;
                }
            }
        }

        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("[ID: {}] Conexão HTTP fechada por {}", id, addr);
                counter!("space_core_total_connections_closed", 1);
                break;
            }
            Ok(n) => {
                counter!("space_core_bytes_read", n as u64);
                debug!("[ID: {}] Lido {} bytes de {} (HTTP)", id, n, addr);

                let mut headers = [httparse::EMPTY_HEADER; 16];
                let mut req = httparse::Request::new(&mut headers);

                match req.parse(&buffer[0..n]) {
                    Ok(httparse::Status::Complete(_bytes_consumed)) => {
                        info!("[ID: {}] Requisição HTTP recebida: Método='{}' Caminho='{}' HTTP/{:?}",
                            id,
                            req.method.unwrap_or(""),
                            req.path.unwrap_or(""),
                            req.version.unwrap_or(0),
                        );
                        debug!("[ID: {}] HTTP Headers: {:?}", id, req.headers);

                        let response: &[u8] = match req.path.unwrap_or("/") {
                            "/health" => b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: text/plain\r\n\r\nOK",
                            "/metrics" => b"HTTP/1.1 200 OK\r\nContent-Length: 25\r\nContent-Type: text/plain\r\n\r\nCheck port 9000 for metrics",
                            _ => b"HTTP/1.1 200 OK\r\nContent-Length: 24\r\nContent-Type: text/plain\r\n\r\nHello from HTTP server!"
                        };

                        if let Err(e) = stream.write_all(response).await {
                             error!("[ID: {}] Erro ao enviar resposta HTTP para {}: {}", id, addr, e);
                             counter!("space_core_write_errors_total", 1);
                             break;
                        }
                        counter!("space_core_bytes_written", response.len() as u64);
                        counter!("space_core_requests_processed_total", 1);
                        info!("[ID: {}] Processada e respondida HTTP para {}", id, addr);
                        break; // Close connection after response
                    },
                    Ok(httparse::Status::Partial) => {
                        debug!("[ID: {}] Requisição HTTP parcial. Precisa ler mais bytes.", id);
                        continue; // Continue reading more data
                    },
                    Err(e) => {
                        warn!("[ID: {}] Erro ao fazer parsing HTTP: {}", id, e);
                        let error_response: &[u8] = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                        if let Err(e) = stream.write_all(error_response).await {
                             error!("[ID: {}] Erro ao enviar erro 400 para {}: {}", id, addr, e);
                        }
                        counter!("space_core_write_errors_total", 1);
                        break;
                    },
                }
            }
            Err(e) => {
                error!("[ID: {}] Erro de leitura da conexão HTTP {}: {}", id, addr, e);
                counter!("space_core_read_errors_total", 1);
                break;
            }
        }
    }
    Ok(())
}