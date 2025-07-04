// Conteúdo para: rs_core/src/bin/server.rs

// Importa a função que criamos na nossa biblioteca `rs_core`.
use rs_core::network::run_server;

// O atributo `#[tokio::main]` transforma a função `main` assíncrona
// em um ponto de entrada síncrono e inicializa o runtime do Tokio.
#[tokio::main]
async fn main() {
    println!("[INFO] Iniciando o servidor Space...");

    // Chama a função principal do nosso módulo de rede e aguarda sua conclusão.
    if let Err(e) = run_server().await {
        // Se a função run_server retornar um erro, imprime o erro na saída de erro padrão.
        eprintln!("Erro no servidor: {}", e);
    }
}