// space/rust_core/space_core/src/metrics.rs

use metrics::{describe_counter, register_counter};
use once_cell::sync::Lazy;
use metrics_exporter_prometheus::PrometheusBuilder;
use anyhow::Context;
use tracing::info;
use std::net::SocketAddr;

// --- Contadores de HTTP ---
// Corrigindo a inicialização dos contadores para compatibilidade com metrics 0.21.1
pub static HTTP_REQUESTS_TOTAL: Lazy<metrics::Counter> = Lazy::new(|| {
    register_counter!("http_requests_total")
});

pub static HTTP_REDIRECTS_TOTAL: Lazy<metrics::Counter> = Lazy::new(|| {
    register_counter!("http_redirects_total")
});

pub static RAW_PACKETS_TOTAL: Lazy<metrics::Counter> = Lazy::new(|| {
    register_counter!("raw_packets_total")
});

pub static RAW_IPV4_PACKETS_TOTAL: Lazy<metrics::Counter> = Lazy::new(|| {
    register_counter!("raw_ipv4_packets_total")
});

pub static RAW_IPV6_PACKETS_TOTAL: Lazy<metrics::Counter> = Lazy::new(|| {
    register_counter!("raw_ipv6_packets_total")
});

// --- Inicialização do Recorder Prometheus ---
pub async fn init_metrics() -> anyhow::Result<()> {
    // Definir as descrições dos contadores explicitamente
    describe_counter!("http_requests_total", "Total HTTP requests received.");
    describe_counter!("http_redirects_total", "Total HTTP redirects issued.");
    describe_counter!("raw_packets_total", "Total raw packets received.");
    describe_counter!("raw_ipv4_packets_total", "Total raw IPv4 packets received.");
    describe_counter!("raw_ipv6_packets_total", "Total raw IPv6 packets received.");

    // Configurar o Prometheus exporter
    PrometheusBuilder::new()
        .with_http_listener("0.0.0.0:9000".parse::<SocketAddr>()?)
        .install()
        .context("Failed to build and install Prometheus recorder")?;

    info!("Prometheus metrics endpoint enabled on 0.0.0.0:9000/metrics");
    Ok(())
}