# Arquitetura do Space Web Server: Uma Visão Geral de Duas Camadas

Este documento descreve a arquitetura fundamental do **Space Web Server**, que é baseada em um design de duas camadas para otimizar o desempenho, a segurança e a flexibilidade. O servidor é dividido em uma camada de baixo nível (Core) implementada em **Rust** e uma camada de alto nível (Aplicação) desenvolvida em **Python**.

---

## 1. Diagrama de Arquitetura de Alto Nível

A seguir, um diagrama em ASCII art que representa visualmente a interação entre as duas camadas e os principais componentes:

```
+-------------------+
|      Cliente      |
| (Navegador, App)  |
+-------------------+
          |
          | (Requisições HTTP/S, FTP, etc.)
          v
+-------------------------------------------------+
|      Camada de Baixo Nível (Core - Rust)        |
+-------------------------------------------------+
| - Escuta em portas (TCP/UDP)                    |
| - Gerenciamento de Sockets & Multiplexação      |
|   (até 100.000 conexões/segundo)                |
| - Manipulação de Buffers de Rede                |
| - Handshake TCP, Timeouts & Keep-Alive          |
| - Segurança de Baixo Nível (Rate Limiting)      |
| - Processamento Inicial de Protocolos:          |
|   - TCP/IP (IPv4/IPv6)                          |
|   - TLS/SSL (HTTPS, FTPS)                       |
|   - Parsing Básico HTTP (Versão, Cabeçalhos)    |
+-------------------------------------------------+
          |
          | (Requisições parseadas, Eventos)
          | (Protocolo IPC: Sockets de Domínio Unix / Loopback TCP + Serialização Binária)
          v
+-------------------------------------------------+
|      Camada de Alto Nível (App - Python)        |
+-------------------------------------------------+
| - Roteamento Inteligente (URLs, APIs)          |
| - Processamento de Cabeçalhos HTTP              |
| - Gerenciamento de Autenticação/Sessão/Cookies  |
| - Servidor de Conteúdo Estático (WSE)           |
| - Servidor de Conteúdo Dinâmico (WSD)           |
|   (Lógica de Negócios, SQL/NoSQL via nativo)    |
| - Adaptadores para Web Frameworks (WFC/WFP)     |
| - Detecção de Ataques (WAF Básico)              |
| - Geração de Respostas HTTP (Status Code)       |
+-------------------------------------------------+
          |
          | (Respostas prontas para envio)
          v
+-------------------------------------------------+
|      Camada de Baixo Nível (Core - Rust)        |
|      (Para enviar resposta ao Cliente)          |
+-------------------------------------------------+
          |
          | (Logs, Métricas)
          v
+-------------------------------------------------+
|      Sistema de Monitoramento/Logs              |
|      (Prometheus/Grafana, ELK - Futuro)         |
+-------------------------------------------------+
```

---

## 2. Responsabilidades de Cada Camada

### 2.1. Camada de Baixo Nível (Core - Rust)

Esta camada, desenvolvida em **Rust**, atua como o **coração** do servidor, sendo responsável por tudo que envolve a **comunicação de rede e I/O de alta performance**. Sua principal função é ser um "proxy" eficiente e seguro, lidando com o tráfego bruto da rede e passando requisições pré-processadas para a camada de alto nível.

* **Ponto de Entrada e Performance:** É o primeiro contato com as requisições que chegam ao servidor. Sua eficiência em Rust garante a capacidade de lidar com um alto volume de conexões simultâneas (visando até 100.000 conexões por segundo) e a minimização da latência.
* **Gerenciamento de Sockets:** Responsável por abrir, fechar e gerenciar as conexões TCP/UDP, utilizando técnicas de **multiplexação de I/O** (como `epoll` ou `kqueue`) para lidar com centenas de milhares de conexões de forma não bloqueante.
* **Controle de Conexão:** Gerencia o ciclo de vida completo das conexões, incluindo o handshake TCP, detecção de inatividade (timeouts) e uso de Keep-Alive para manter conexões ativas.
* **Manipulação de Buffers:** Otimiza a alocação e o reuso de buffers de rede para leituras e escritas, reduzindo cópias de dados e o consumo de memória.
* **Processamento de Protocolos de Transporte:** Realiza a decapsulação e o parsing de protocolos de rede de baixo nível, como **TCP/IP** (suporte a IPv4 e IPv6) e a camada de segurança **TLS/SSL**.
* **Parsing Básico de HTTP:** Faz um parsing inicial dos cabeçalhos HTTP para identificar a versão do protocolo e outras informações cruciais antes de encaminhar a requisição.
* **Segurança de Rede:** Implementa defesas essenciais a nível de rede, como **rate limiting** de novas conexões para mitigar ataques DoS (Denial of Service) básicos.
* **Encaminhamento de Dados:** Transfere requisições parseadas para a camada Python e recebe as respostas processadas para enviar de volta aos clientes.

### 2.2. Camada de Alto Nível (App - Python)

Construída em **Python 3.11.3**, esta camada é o **cérebro da aplicação**, onde a lógica de negócios e o processamento de conteúdo são realizados. Ela recebe as requisições "limpas" e pré-processadas da camada Rust, foca na inteligência da aplicação e gera as respostas completas.

* **Lógica de Negócios Central:** Gerencia o roteamento de requisições, o processamento de conteúdo estático e dinâmico, e a interação com outros serviços (como bancos de dados).
* **Roteamento Inteligente:** Possui um sistema de roteamento sofisticado que pode diferenciar entre vários tipos de URLs (API, estáticas, dinâmicas, segmentadas, raiz, etc.) e direcionar a requisição para o módulo Python apropriado.
* **Processamento HTTP Completo:** Realiza o parsing completo de cabeçalhos HTTP, manipulação de cookies, gerenciamento de sessões e autenticação (por exemplo, via JWT com módulos nativos).
* **Serviço de Conteúdo:**
    * **Websites Estáticos (WSE):** Responsável por servir arquivos estáticos (HTML, CSS, JavaScript, imagens, etc.) de forma otimizada, incluindo suporte a cache HTTP.
    * **Websites Dinâmicos (WSD):** Executa a lógica de aplicação Python para gerar conteúdo dinâmico, interagindo com dados (bancos de dados SQL/NoSQL - via módulos nativos ou reimplementações) e outras APIs.
* **Extensibilidade e Adaptação:** Fornece interfaces para que frameworks web Python customizados (WFC) ou populares (WFP, via adaptadores WSGI/ASGI implementados nativamente) possam ser integrados e utilizados, tornando o Space Server um host versátil.
* **Segurança de Aplicação:** Implementa um firewall de aplicação básico (WAF), incluindo validação de entrada, proteção contra XSS, CSRF, Path Traversal, e outras vulnerabilidades comuns.
* **Geração de Respostas:** Constrói as respostas HTTP finais (incluindo cabeçalhos e corpo) e as envia de volta para a camada Rust para transmissão ao cliente.

---

## 3. Comunicação e Interconexão entre as Camadas

A comunicação eficiente entre as camadas Rust e Python é o pilar desta arquitetura. A escolha do mecanismo de Inter-Process Communication (IPC) e do formato de serialização é crucial para o desempenho.

* **Mecanismo de IPC:**
    * A opção preferencial é **Sockets de Domínio Unix** para ambientes Linux, devido à sua alta performance e baixo overhead para comunicação no mesmo host.
    * Alternativamente, um protocolo customizado sobre **TCP Loopback** pode ser considerado para maior flexibilidade ou em cenários onde Sockets de Domínio Unix não são ideais.
* **Protocolo de Mensagens:**
    * A comunicação será baseada em um protocolo de mensagens leve e bem definido.
    * As mensagens serão **serializadas em um formato binário eficiente**, como `bincode`, `FlatBuffers` ou `Protocol Buffers`. Isso minimiza o tamanho das mensagens e o tempo de parsing, garantindo uma troca de dados rápida entre as linguagens.
* **Fluxo de Dados:**
    * A camada Rust recebe uma requisição, realiza seu processamento de baixo nível e a serializa em uma mensagem para a camada Python.
    * A camada Python desserializa a mensagem, processa a lógica de negócios e gera uma resposta.
    * A resposta é então serializada pela camada Python e enviada de volta para a camada Rust.
    * A camada Rust desserializa a resposta, reencapsula-a nos protocolos de rede e a envia de volta ao cliente original.

---

## 4. Logging, Monitoramento e Telemetria

Ambas as camadas contribuirão para um sistema centralizado de logs e monitoramento. Eventos importantes (conexões, erros, avisos) e métricas de desempenho (uso de recursos, throughput, latência) serão coletados. Futuramente, isso permitirá a integração com ferramentas como Prometheus para métricas e Grafana para visualização, ou um ELK Stack para análise de logs, fornecendo insights abrangentes sobre o comportamento e a saúde do servidor.

---

Esta arquitetura de duas camadas visa combinar a performance e o controle de baixo nível do Rust com a flexibilidade, a produtividade e o vasto ecossistema de bibliotecas de alto nível do Python, resultando em um servidor web robusto, seguro e escalável.
