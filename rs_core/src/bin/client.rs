// Conteúdo para: rs_core/src/bin/client.rs

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[CLIENTE] Conectando ao servidor...");

    // 1. Tenta conectar ao nosso servidor
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("[CLIENTE] Conectado!");

    // 2. Cria um leitor com buffer para ler uma linha inteira
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    // 3. Tenta ler dados do servidor até encontrar uma nova linha '\n'
    println!("[CLIENTE] Aguardando resposta do servidor...");
    match reader.read_line(&mut line).await {
        Ok(0) => {
            println!("[CLIENTE] Servidor fechou a conexão sem enviar dados.");
        }
        Ok(_n) => {
            // Remove a quebra de linha e imprime a resposta
            print!("[CLIENTE] Resposta recebida: {}", line.trim_end());
        }
        Err(e) => {
            eprintln!("[CLIENTE] Erro ao ler do servidor: {}", e);
        }
    }

    println!("\n[CLIENTE] Teste concluído.");
    Ok(())
}