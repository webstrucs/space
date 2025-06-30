# Robustez e Gerenciamento de Recursos no Space Core (Camada Rust)

Este documento detalha as considerações e estratégias implementadas na camada de baixo nível (Rust) do projeto Space Core para garantir robustez, gerenciamento eficiente de recursos e um tratamento de erros consistente.

## 1. Gerenciamento de Memória

Rust é conhecido por sua segurança de memória sem a necessidade de um coletor de lixo (Garbage Collector). Isso é alcançado através de um sistema de posse (ownership) e borrowing (empréstimo) em tempo de compilação.

* **Ownership:** No Space Core, a maior parte do gerenciamento de memória é intrínseco ao modelo de ownership do Rust. Variáveis têm um proprietário, e quando o proprietário sai do escopo, a memória associada é automaticamente liberada. Isso previne vazamentos de memória e erros de "double free".
* **Lifetimes:** Para referências, Rust garante que elas sejam válidas pelo tempo que forem usadas, prevenindo "dangling pointers".
* **Async/Await e `tokio`:** O framework assíncrono `tokio` otimiza o uso de recursos ao permitir a criação de tarefas leves (green threads) que usam cooperative scheduling. Isso significa que as tarefas cedem o controle voluntariamente, permitindo que múltiplas operações de I/O ocorram sem a necessidade de múltiplos threads de sistema operacional pesados, resultando em menor consumo de memória e CPU para lidar com um grande número de conexões.
* **Clones Explícitos:** Em cenários de concorrência (como com `mpsc::Sender` e `CancellationToken`), o Rust exige clonagem explícita (``.clone()``) para compartilhar a posse de dados entre tarefas. Embora o ``.clone()`` para tipos como `Arc` (Atomic Reference Counted) incremente um contador de referências e não faça uma cópia profunda dos dados, ele adiciona uma pequena sobrecarga. No Space Core, isso é feito de forma controlada, garantindo que cada tarefa tenha acesso aos recursos de que precisa sem comprometer a segurança da memória.

## 2. Política de Tratamento de Erros

A gestão de erros no Space Core prioriza a robustez, a rastreabilidade e a categorização da gravidade do problema.

* **`anyhow::Result`:** Utilizamos a crate `anyhow` para tratamento de erros. Ela fornece uma maneira simples e flexível de propagar erros através de `Result` e adicionar contexto (``.context()``) às falhas. Isso é fundamental para depuração, pois um erro distante da sua origem pode carregar todo o rastro de eventos que levaram a ele.
    * **Exemplo de Uso:**
        ```rust
        // Erro original
        let result = some_io_operation().context("Falha ao ler dados da rede")?;
        // Propagação com mais contexto
        let processed_data = process_data(result).context("Falha ao processar dados recebidos")?;
        ```
* **Registro de Erros (Logging com `tracing`):** A crate `tracing` é usada para instrumentação e logging. Erros são logados com a macro `error!`, e `anyhow` se integra bem a isso, permitindo que a cadeia de contexto seja impressa de forma legível.
    * **``if let Err(e) = ...``:** Para operações "fire-and-forget" (onde o envio de uma mensagem em um canal pode falhar, mas o programa não deve parar), utilizamos ``if let Err(e) = ... { error!("{:?}", e); }`` para logar o erro sem abortar a tarefa ou o programa principal.
* **Severidade de Erro (`ErrorSeverity` Protobuf):** As mensagens de erro enviadas via IPC (para a camada Python ou para o log) incluem um campo `severity` (INFO, WARNING, ERROR, CRITICAL). Isso permite que a camada de alto nível ou sistemas de monitoramento interpretem a gravidade do problema e tomem ações adequadas (ex: disparar um alerta crítico para um erro `CRITICAL`).

## 3. Graceful Shutdown (Desligamento Elegante)

Um `graceful shutdown` é essencial para garantir que o servidor encerre suas operações de forma limpa, minimizando a perda de dados e evitando estados inconsistentes.

* **`tokio-util::sync::CancellationToken`:** Implementamos um mecanismo de `CancellationToken` (token de cancelamento) para sinalizar o encerramento em todas as tarefas assíncronas em execução.
    * **Sinalização Unificada:** Um token "raiz" é criado no `main.rs`.
    * **Propagação:** Clones deste token são passados para cada tarefa (`tokio::spawn`) que precisa estar ciente do desligamento.
    * **Escuta Ativa:** Dentro dos loops de cada tarefa (ex: `component_manager`, `status_monitor`, listeners HTTP/HTTPS, processador de pacotes), `tokio::select!` é usado para priorizar o recebimento do sinal de cancelamento (``_ = shutdown_token.cancelled() => { break; }``). Isso permite que as tarefas interrompam suas operações em andamento e saiam de seus loops de forma controlada.
* **Triggers de Shutdown:** O sinal de shutdown pode ser iniciado por:
    * **`Ctrl+C`:** Capturado diretamente no `main.rs`.
    * **Comando IPC `Shutdown`:** Enviado pela camada Python via Unix Domain Socket.
    * **Falha Crítica de Tarefa:** Se uma das tarefas principais do servidor falhar inesperadamente (ex: um listener de rede parar), o `tokio::select!` no `main` captura essa falha e também aciona o `CancellationToken` global, iniciando um desligamento elegante.
* **Limpeza de Recursos:** No caso do servidor IPC, o arquivo do socket (``/tmp/space_core_ipc.sock``) é explicitamente removido durante o processo de desligamento, garantindo uma limpeza completa.

## 4. Monitoramento de Recursos

O Space Core integra métricas Prometheus para observabilidade.

* **Contadores:** Métricas como `HTTP_REQUESTS_TOTAL`, `HTTP_REDIRECTS_TOTAL`, `RAW_PACKETS_TOTAL`, `RAW_IPV4_PACKETS_TOTAL`, `RAW_IPV6_PACKETS_TOTAL` são incrementadas em pontos chave do código. Isso permite monitorar a carga e o comportamento do sistema.
* **Exposição de Métricas:** As métricas são expostas em um endpoint Prometheus (``0.0.0.0:9000``), permitindo que ferramentas como Grafana possam coletá-las e visualizá-las para uma análise detalhada de uso de CPU, memória, conexões ativas, etc.

Esta combinação de recursos do Rust, bibliotecas como `tokio` e `anyhow`, e a implementação cuidadosa de um mecanismo de `graceful shutdown` e monitoramento de métricas, garante que o Space Core seja uma base robusta e observável para o desenvolvimento contínuo.
