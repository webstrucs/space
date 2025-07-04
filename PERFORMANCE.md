# Guia de Performance e Benchmarking

Este documento descreve como realizar testes de carga e profiling de performance no servidor Space.

## 1. Compilando para Performance

Para todos os testes e execuções em produção, o servidor deve ser compilado em **modo Release** para ativar as otimizações do compilador Rust.

```bash
# A partir da pasta rs_core
cargo build --release --bin server
```
O executável final estará em `rs_core/target/release/server`.

## 2. Teste de Carga com `oha`

Utilizamos a ferramenta `oha` para realizar testes de carga de conexões TCP.

**Instalação:**
```bash
cargo install oha
```

**Execução do Teste:**
1.  Inicie o servidor em modo release: `cargo run --release --bin server`.
2.  Em outro terminal, execute o `oha`:
    ```bash
    # Simula 50 clientes por 10 segundos
    oha -c 50 -z 10s --no-tui tcp://127.0.0.1:8080
    ```

## 3. Profiling de CPU com `flamegraph`

Utilizamos `cargo-flamegraph` para identificar gargalos de performance a nível de CPU.

**Instalação:**
```bash
# Pré-requisito no Debian/Ubuntu
sudo apt install -y linux-perf

# Instalação da ferramenta
cargo install flamegraph
```

**Execução do Profiler:**
1.  Garanta que o `debug = true` está configurado para o perfil `[profile.release]` no `Cargo.toml`.
2.  Garanta que o `kernel.perf_event_paranoid` está configurado com um valor apropriado (ex: `1`).
3.  Execute o servidor através do `flamegraph`:
    ```bash
    # A partir da pasta rs_core
    cargo flamegraph --release --bin server
    ```
4.  Enquanto o profiler está rodando, gere carga usando o `oha` em outro terminal.
5.  Pare o servidor (`Ctrl + C`) para que o arquivo `flamegraph.svg` seja gerado.