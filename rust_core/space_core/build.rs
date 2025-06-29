// space/rust_core/space_core/build.rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Especifica onde o arquivo .proto está localizado
    prost_build::compile_protos(
        &["proto/messages.proto"], // Caminho para o seu arquivo .proto
        &["proto/"],               // Diretório base para procurar arquivos .proto
    )?;
    Ok(())
}