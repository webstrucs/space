# Estágio 1: Base com Rust
FROM rust:1.79 as builder

WORKDIR /usr/src/space
COPY . .

# Instala dependências e faz o build inicial para cache
RUN cargo build --release

# Estágio 2: Ambiente de desenvolvimento final com Python
FROM python:3.11-slim

WORKDIR /usr/src/app

# Copia os artefatos do build Rust do estágio anterior
COPY --from=builder /usr/src/space/target/release/space .
COPY . .

# Instala dependências Python
RUN pip install --no-cache-dir -e .

# Expõe a porta padrão do servidor
EXPOSE 8000

# Mantém o container rodando
CMD ["tail", "-f", "/dev/null"]