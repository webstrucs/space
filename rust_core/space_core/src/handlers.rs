// space/rust_core/space_core/src/handlers.rs

use hyper::service::service_fn;
use hyper::{Body, Request, Response, StatusCode};
use tokio::net::TcpStream;
use tokio_rustls::TlsStream;
use std::net::SocketAddr;
use tracing::{info, error, debug};
use anyhow::{Result, Context};

// Importar o módulo de métricas para acessar os contadores
use crate::metrics;

// Função para lidar com requisições HTTP
pub async fn handle_http_connection(stream: TcpStream, peer_addr: SocketAddr) -> Result<()> {
    info!("Handling HTTP connection from: {}", peer_addr);

    let service = service_fn(move |req| {
        handle_request(req, peer_addr)
    });

    if let Err(e) = hyper::server::conn::Http::new()
        .serve_connection(stream, service)
        .await
        .context(format!("HTTP connection handler for {} failed", peer_addr)) {
        error!("{:?}", e);
        return Err(e);
    }

    info!("HTTP connection from {} closed.", peer_addr);
    Ok(())
}

// Função para lidar com requisições HTTPS
pub async fn handle_https_connection(stream: TlsStream<TcpStream>, peer_addr: SocketAddr) -> Result<()> {
    info!("Handling HTTPS connection from: {}", peer_addr);

    let service = service_fn(move |req| {
        handle_request(req, peer_addr)
    });

    if let Err(e) = hyper::server::conn::Http::new()
        .serve_connection(stream, service)
        .await
        .context(format!("HTTPS connection handler for {} failed", peer_addr)) {
        error!("{:?}", e);
        return Err(e);
    }

    info!("HTTPS connection from {} closed.", peer_addr);
    Ok(())
}

// Função genérica para processar a requisição (HTTP ou HTTPS)
async fn handle_request(req: Request<Body>, peer_addr: SocketAddr) -> Result<Response<Body>, hyper::Error> {
    debug!("Received request from {}: Method: {}, URI: {}", peer_addr, req.method(), req.uri());

    // Incrementar o contador de requisições HTTP/HTTPS totais
    metrics::HTTP_REQUESTS_TOTAL.increment(1);

    let response_body = format!("Hello from Space Core! You requested: {}", req.uri());

    // Exemplo de erro simulado (apenas para demonstração)
    if req.uri().path() == "/error" {
        error!("Simulating an internal server error for request from {}", peer_addr);
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Internal Server Error"))
            .unwrap());
    }
    // Exemplo de redirecionamento simulado para testar o contador de redirecionamentos
    else if req.uri().path() == "/redirect" {
        info!("Simulating a redirect for request from {}", peer_addr);
        metrics::HTTP_REDIRECTS_TOTAL.increment(1); // Incrementar o contador de redirecionamentos
        return Ok(Response::builder()
            .status(StatusCode::FOUND) // 302 Found
            .header(hyper::header::LOCATION, "/new-location")
            .body(Body::from("Redirecting to /new-location"))
            .unwrap());
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "text/plain")
        .body(Body::from(response_body))
        .unwrap())
}
