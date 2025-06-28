// space/rust_core/space_core/src/handlers.rs

use hyper::{Body, Response, StatusCode, Request};
use hyper::server::conn::Http;
use tokio::net::TcpStream;
use tracing::{info, debug, error};

use crate::metrics;

// Handler para conexões HTTP
pub async fn handle_http_connection(stream: TcpStream, peer_addr: std::net::SocketAddr) {
    info!("Handling HTTP connection from: {}", peer_addr);

    metrics::HTTP_REQUESTS_TOTAL.increment(1);

    let service = hyper::service::service_fn(move |req: Request<Body>| {
        async move {
            debug!("Received HTTP request: {} {}", req.method(), req.uri());

            let response = if req.uri().path() == "/redirect" {
                metrics::HTTP_REDIRECTS_TOTAL.increment(1);
                Response::builder()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header("Location", "https://github.com/rust-lang/rust")
                    .body(Body::empty())
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(format!("Hello from HTTP! Your IP: {}", peer_addr)))
                    .unwrap()
            };
            Ok::<_, hyper::Error>(response)
        }
    });

    if let Err(err) = Http::new().serve_connection(stream, service).await {
        error!("Error serving HTTP connection from {}: {}", peer_addr, err);
    }
}

// Handler para conexões HTTPS (usa tokio_rustls::server::TlsStream)
pub async fn handle_https_connection(stream: tokio_rustls::server::TlsStream<TcpStream>, peer_addr: std::net::SocketAddr) {
    info!("Handling HTTPS connection from: {}", peer_addr);

    metrics::HTTP_REQUESTS_TOTAL.increment(1);

    let service = hyper::service::service_fn(move |req: Request<Body>| {
        async move {
            debug!("Received HTTPS request: {} {}", req.method(), req.uri());
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(format!("Hello from HTTPS! Your IP: {}", peer_addr)))
                .unwrap();
            Ok::<_, hyper::Error>(response)
        }
    });

    if let Err(err) = Http::new().serve_connection(stream, service).await {
        error!("Error serving HTTPS connection from {}: {}", peer_addr, err);
    }
}