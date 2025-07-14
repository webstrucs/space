# --- Estágio 1: Builder do Rust ---
# Usamos uma imagem oficial do Rust baseada em Debian Bookworm para compilar nosso núcleo.
FROM rust:1.79-slim-bookworm as rust-builder

# Define o diretório de trabalho dentro do contêiner.
WORKDIR /usr/src/space

# Copia apenas os arquivos de dependência primeiro para aproveitar o cache do Docker.
COPY ./rs_core/Cargo.toml ./rs_core/Cargo.lock ./rs_core/
RUN mkdir -p ./rs_core/src && echo "fn main() {}" > ./rs_core/src/main.rs
RUN cargo build --release --bin server

# Copia o resto do código fonte do Rust.
COPY ./rs_core ./rs_core

# Compila nosso servidor em modo release, garantindo que as dependências já foram baixadas.
# A flag --locked garante que as versões do Cargo.lock sejam usadas.
RUN cargo build --release --locked --bin server


# --- Estágio 2: Imagem Final de Produção ---
# Começamos com uma imagem Python enxuta (slim), também baseada em Debian Bookworm.
FROM python:3.11-slim-bookworm

# Define o diretório de trabalho da aplicação.
WORKDIR /app

# Cria um usuário não-root para rodar a aplicação por segurança.
RUN useradd --system --create-home appuser
USER appuser
WORKDIR /home/appuser/app

# Copia os arquivos da aplicação Python para a imagem.
COPY --chown=appuser:appuser . .

# Copia APENAS o binário compilado do Rust do estágio de build anterior.
# Isso mantém a imagem final pequena, sem toda a toolchain do Rust.
COPY --from=rust-builder --chown=appuser:appuser /usr/src/space/rs_core/target/release/server .

# Expõe as portas que nosso servidor utiliza.
EXPOSE 8080
EXPOSE 9090

# Define o comando para iniciar o servidor.
# Usaremos um script de entrypoint para iniciar os dois processos (Rust e Python).
ENTRYPOINT ["./entrypoint.sh"]