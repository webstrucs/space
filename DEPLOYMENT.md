# Guia de Deploy e Contêinerização do Servidor Space

Este documento descreve a estratégia e as melhores práticas para fazer o deploy do Servidor Space em um ambiente de produção.

## 1. Estratégia de Contêinerização com Docker

A aplicação é empacotada como uma imagem Docker para garantir consistência e portabilidade entre os ambientes.

-   **Dockerfile Multi-Stage:** Utilizamos um `Dockerfile` de múltiplos estágios para otimizar o tamanho da imagem final.
    1.  **Estágio `rust-builder`:** Um contêiner temporário com a toolchain completa do Rust é usado para compilar o núcleo (`rs_core`) em modo `release`.
    2.  **Estágio Final:** Uma imagem Python minimalista (`slim`) é usada como base. Apenas o binário Rust compilado é copiado do estágio anterior, e o código Python é adicionado. Isso resulta em uma imagem final leve, sem as ferramentas de compilação do Rust.

-   **Usuário Não-Root:** Por segurança, a aplicação dentro do contêiner é executada por um usuário do sistema com privilégios limitados (`appuser`), em vez de `root`.

-   **Entrypoint:** Um script `entrypoint.sh` é usado para orquestrar a inicialização dos dois processos necessários: o servidor Python (IPC) em segundo plano e o servidor Rust (Gateway) em primeiro plano.

## 2. Melhores Práticas de Deploy em Produção

### 2.1. Configuração
-   Toda a configuração (porta, host, caminhos, etc.) é gerenciada via **variáveis de ambiente**, conforme implementado na Issue #014. Isso permite configurar a aplicação para diferentes ambientes (staging, produção) sem alterar o código ou a imagem Docker.
-   **Exemplo:** `docker run -e PORT=80 -e WORKERS=16 ...`

### 2.2. Logging
-   A aplicação está configurada para logar no formato de **texto estruturado para a saída padrão (`stdout`/`stderr`)**. Esta é a melhor prática para ambientes de contêineres, pois permite que orquestradores (Docker, Kubernetes) capturem, agreguem e gerenciem os logs de forma centralizada. **Não se deve logar para arquivos dentro do contêiner.**

### 2.3. Persistência de Dados
Dados que devem sobreviver ao ciclo de vida do contêiner precisam ser montados como **volumes**.
-   **Banco de Dados:** O arquivo `space_database.db` deve ser armazenado em um volume Docker para não ser perdido quando o contêiner for recriado. Ex: `-v /path/no/host/data:/home/appuser/app/`.
-   **Conteúdo dos Sites (`works/`):** O diretório `works/` também deve ser um volume para que os sites estáticos e dinâmicos possam ser gerenciados externamente.

### 2.4. Health Checks
Para que orquestradores como Kubernetes ou Docker Swarm saibam se a nossa aplicação está saudável, um health check deve ser configurado.
-   **Estratégia:** Um endpoint HTTP de baixo custo, como `/health`, deve ser adicionado à aplicação. Por enquanto, um "TCP health check" na porta principal (8080) é suficiente.
-   **Exemplo no `Dockerfile`:**
    ```dockerfile
    HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
      CMD curl --fail http://localhost:8080/ || exit 1
    ```
    *(Nota: Este exemplo assume um endpoint HTTP, mas a ideia pode ser adaptada para um check de conexão TCP).*

## 3. Otimizações da Imagem Docker

-   **Imagens Base Mínimas:** Já utilizamos as tags `-slim` para as imagens Rust e Python, que são significativamente menores que as padrão.
-   **Arquivo `.dockerignore`:** Impede que arquivos desnecessários (como `.git`, `target/` local) sejam enviados ao daemon do Docker, otimizando o tempo de build e o tamanho da imagem.
-   **Cache de Camadas:** A ordem dos comandos no `Dockerfile` foi pensada para otimizar o cache, copiando arquivos de dependência que mudam pouco (`Cargo.toml`) antes do código-fonte, que muda com frequência.
-   **Otimização Futura (Compilação Estática):** Para imagens ainda menores, o binário Rust pode ser compilado de forma estática usando a `musl` toolchain. Isso elimina a dependência da `glibc` e permite usar imagens base ainda mais enxutas, como `scratch` ou `alpine`.