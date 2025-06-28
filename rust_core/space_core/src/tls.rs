// space/rust_core/space_core/src/tls.rs

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, PrivatePkcs1KeyDer, PrivateSec1KeyDer};
use rustls_pemfile::{read_one, Item};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use std::io::Cursor;
use anyhow::Context;
use tracing::warn;

pub async fn load_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<ServerConfig> {
    // 1. Carregar o arquivo de certificado
    let mut cert_file = File::open(cert_path).await
        .context(format!("Failed to open certificate file: {}", cert_path))?;
    let mut cert_bytes = Vec::new();
    cert_file.read_to_end(&mut cert_bytes).await?;

    // 2. Analisar os certificados
    let certs: Vec<CertificateDer<'static>> = {
        let mut reader = Cursor::new(cert_bytes);
        rustls_pemfile::certs(&mut reader)
            .filter_map(|r| r.ok())
            .map(|der_vec| der_vec.into_owned())
            .collect()
    };

    if certs.is_empty() {
        return Err(anyhow::anyhow!("No certificates found in {}", cert_path));
    }

    // 3. Carregar o arquivo de chave privada
    let mut key_file = File::open(key_path).await
        .context(format!("Failed to open private key file: {}", key_path))?;
    let mut key_bytes = Vec::new();
    key_file.read_to_end(&mut key_bytes).await?;

    // 4. Analisar a chave privada
    let mut keys: Vec<PrivateKeyDer<'static>> = Vec::new();
    let mut reader = Cursor::new(key_bytes);

    // CORREÇÃO: Usar while let em vez de for loop para evitar warning
    while let Some(item) = read_one(&mut reader)? {
        match item {
            Item::Pkcs1Key(key_der_vec) => {
                keys.push(PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key_der_vec)));
            },
            Item::Pkcs8Key(key_der_vec) => {
                keys.push(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der_vec)));
            },
            Item::Sec1Key(key_der_vec) => {
                keys.push(PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(key_der_vec)));
            },
            _ => continue,
        }
    }

    if keys.is_empty() {
        return Err(anyhow::anyhow!("No private keys found in {}", key_path));
    }
    if keys.len() > 1 {
        warn!("Multiple private keys found in {}, using the first one.", key_path);
    }

    // 5. Construir a configuração do servidor Rustls
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .context("Failed to build server config from certificates and key")?;

    Ok(config)
}