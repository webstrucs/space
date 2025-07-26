# 🚀 Guia de Configuração do Ambiente de Desenvolvimento - Space

Este documento fornece um guia completo para configurar o ambiente de desenvolvimento para o projeto Space. O objetivo é que qualquer desenvolvedor consiga ter um ambiente funcional em menos de 30 minutos.

Existem duas maneiras principais de configurar o ambiente:
1.  **Localmente (Nativo):** Instalando as ferramentas Rust e Python diretamente na sua máquina.
2.  **Com Docker:** Usando um ambiente conteinerizado que já inclui todas as dependências. (Recomendado para consistência)

---

## Opção 1: Configuração Local (Nativa)

### 1. Requisitos da Toolchain Rust

O Space utiliza a versão mais recente e estável do Rust.

- **Instalador:** `rustup` (o gerenciador oficial de toolchains do Rust).
- **Instalação:** Se você não tem o Rust instalado, siga as instruções no site oficial: [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
- **Verificação:** Após a instalação, verifique se os comandos estão funcionando:
  ```bash
  rustc --version
  cargo --version
  ```

### 2. Configuração do Ambiente Python

- **Versão do Python:** Recomenda-se Python 3.8 ou superior.
- **Ambiente Virtual:** É **altamente recomendado** usar um ambiente virtual para isolar as dependências do projeto.

  ```bash
  # 1. Crie um ambiente virtual na pasta do projeto
  python3 -m venv venv

  # 2. Ative o ambiente virtual
  # No macOS / Linux:
  source venv/bin/activate
  # No Windows:
  .\venv\Scripts\activate

  # 3. Instale as dependências
  pip install -e .
  ```

### 3. Configurações de IDE/Editor (Recomendado: VS Code)

Para uma experiência de desenvolvimento ideal, recomendamos o Visual Studio Code com as seguintes extensões:

- **`rust-analyzer`**: Essencial para desenvolvimento em Rust (autocompletar, análise de código, etc.).
- **`Python (ms-python.python)`**: Suporte oficial da Microsoft para desenvolvimento em Python.
- **`Even Better TOML`**: Melhora a sintaxe de arquivos `.toml` como `Cargo.toml` e `pyproject.toml`.
- **`CodeLLDB`**: Para depuração (debugging) do código Rust.

#### Configuração de Debugging (launch.json)

Crie o arquivo `.vscode/launch.json` com a seguinte configuração para depurar o core Rust:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug executable 'space'",
      "cargo": {
        "args": [
          "build",
          "--bin=space",
          "--package=space"
        ],
        "filter": {
          "name": "space",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

---

## Opção 2: Ambiente de Desenvolvimento com Docker (Recomendado)

Usar Docker garante um ambiente consistente para todos os desenvolvedores.

### 1. Crie o `Dockerfile`

Na raiz do projeto, crie um arquivo chamado `Dockerfile`:

```dockerfile
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
```

### 2. Crie o `docker-compose.yml`

Para facilitar o gerenciamento do container, crie um arquivo `docker-compose.yml`:

```yaml
version: '3.8'
services:
  dev:
    build: .
    volumes:
      - .:/usr/src/app
    ports:
      - "8000:8000"
    command: tail -f /dev/null
```

### 3. Como Usar

- **Construir e iniciar o container:**
  ```bash
  docker-compose up -d --build
  ```
- **Acessar o terminal do container:**
  ```bash
  docker-compose exec dev bash
  ```
- Dentro do container, você terá acesso a `cargo`, `python` e a todo o código-fonte, que está sincronizado com sua máquina local.

---

## Scripts de Desenvolvimento

Para simplificar os comandos comuns, você pode criar um `Makefile` na raiz do projeto.

Crie um arquivo `Makefile` com o seguinte conteúdo:

```makefile
.PHONY: build test-rust test-python benchmark run

# Constrói o projeto Rust em modo de desenvolvimento
build:
	cargo build

# Executa os testes do core Rust
test-rust:
	cargo test --verbose

# Executa os testes da API Python
test-python:
	python -m pytest

# Executa os benchmarks do core Rust
benchmark:
	cargo bench --verbose

# Executa o projeto (assume que o binário se chama 'space')
run:
	cargo run
```

Agora você pode usar comandos simples como:
- `make build`
- `make test-rust`
- `make test-python`
- `make run`