# Política de Segurança do Servidor Space

Este documento descreve as medidas de segurança implementadas na camada de baixo nível (`rs_core`) do servidor.

## Mitigação de Ataques de Negação de Serviço (DoS)

### 1. Rate Limiting de Conexões
- **Mecanismo:** O servidor implementa um limitador de taxa para novas conexões TCP.
- **Lógica:** É mantido um registro de timestamps de conexões por endereço IP. Um IP que exceder um número máximo de conexões (`RATE_LIMIT_MAX_CONN`) dentro de uma janela de tempo (`RATE_LIMIT_WINDOW_SECS`) terá as novas conexões recusadas até que a taxa de requisições diminua.
- **Objetivo:** Prevenir que um único ator malicioso esgote os recursos do servidor (como descritores de arquivo) abrindo um número excessivo de conexões rapidamente.

### 2. Mitigação de SYN Flood
- **Mecanismo:** A proteção primária contra ataques de SYN Flood é delegada à pilha TCP/IP do sistema operacional (Linux Kernel).
- **Justificativa:** O kernel possui mecanismos robustos e eficientes (como SYN cookies) para lidar com esse tipo de ataque antes mesmo que as conexões cheguem à nossa aplicação. Nossa lógica de rate limiting atua como uma segunda camada de defesa para as conexões que são completamente estabelecidas.

### 3. Timeouts e Keep-Alive
- **Mecanismo:** O servidor utiliza timeouts de inatividade e a opção TCP Keep-Alive.
- **Objetivo:** Liberar recursos de conexões que se tornaram inativas ou foram interrompidas abruptamente, evitando o esgotamento de recursos por "conexões zumbis".

## Segurança de Memória

### Prevenção de Buffer Overflow
- **Mecanismo:** A manipulação de buffers de rede é realizada utilizando a crate `bytes` e sua estrutura `BytesMut`.
- **Garantia de Segurança:** `BytesMut` é um tipo de buffer inteligente que gerencia seu próprio crescimento e capacidade de forma segura. As funções de I/O do Tokio (`read_buf`) são projetadas para interagir com esta estrutura, eliminando o risco de escrita fora dos limites da memória alocada, que é a causa raiz das vulnerabilidades de buffer overflow. A segurança é garantida pelo design da abstração, um pilar da filosofia do Rust.

## Isolamento de Processos

- **Diretriz:** Para ambientes de produção, recomenda-se que o processo do servidor seja executado com o menor privilégio de usuário possível. O uso de técnicas de sandboxing como `chroot` ou contêineres (Docker) é fortemente encorajado para limitar o acesso do processo ao sistema de arquivos e outros recursos do sistema.

---

# Política de Segurança do Servidor Space

Este documento descreve as políticas e implementações de segurança adotadas no projeto Space. A nossa filosofia é a de **defesa em profundidade**, aplicando medidas de segurança em todas as camadas da arquitetura.

## 1. Nível de Sistema Operacional e Rede

A primeira barreira de defesa é a configuração correta do ambiente do servidor.

#### **1.1. Firewall (NFTables)**
A política de firewall é de **negar por padrão (`policy drop`)**. Apenas o tráfego para portas explicitamente permitidas é aceito:
-   **Portas Permitidas:** `22` (SSH), `8080` (Servidor Space), `9090` (Métricas).
-   **Regras Adicionais:** Permite tráfego local (`loopback`) e o retorno de conexões já estabelecidas.
-   **Mitigação de DoS:** Novas conexões para serviços críticos são limitadas a uma taxa de `5/minuto` por IP de origem.

#### **1.2. Acesso Administrativo (SSH)**
O acesso ao servidor é restrito:
-   **Autenticação:** Exclusivamente por chaves SSH.
-   **Login por Senha e `root`:** Desabilitados.

## 2. Nível de Transporte

#### **2.1. Criptografia Fim-a-Fim (TLS)**
Toda a comunicação entre clientes e o servidor é obrigatoriamente criptografada utilizando **TLS**, implementado através da biblioteca `rustls`.

## 3. Nível de Aplicação

#### **3.1. Mitigação de Abuso e DoS**
-   **Rate Limiting de Conexões:** O núcleo Rust implementa um limitador de taxa para novas conexões, agindo como uma segunda camada de defesa.
-   **Timeouts e Keep-Alive:** Conexões ociosas ou "zumbis" são encerradas para preservar recursos.

#### **3.2. Segurança de Acesso e Memória**
-   **Prevenção de Buffer Overflow:** Garantida pelo design da linguagem Rust.
-   **Prevenção de Path Traversal:** A camada Rust inspeciona os bytes brutos em busca de sequências `..`. Como defesa em profundidade, o handler de arquivos em Python resolve o caminho canônico e valida que ele pertence ao diretório raiz permitido.

#### **3.3. Segurança de Conteúdo e Sessão**
-   **WAF (SQL Injection):** Um WAF em Python inspeciona os dados da requisição em busca de padrões de SQLi.
-   **Cabeçalhos HTTP de Segurança:**
    -   `Content-Security-Policy: default-src 'self'`: Previne XSS.
    -   `X-Frame-Options: DENY`: Previne Clickjacking.
    -   `X-Content-Type-Options: nosniff`: Previne ataques de MIME-sniffing.
-   **Cookies Seguros:** Cookies de sessão são gerados com as flags `HttpOnly` e `SameSite=Lax` para proteger contra XSS e CSRF.
-   **Autenticação JWT:** APIs são protegidas com JSON Web Tokens assinados via HMAC-SHA256 e com tempo de expiração.

## 4. Nível de Banco de Dados
-   **Prevenção de SQL Injection:** Uso obrigatório de **queries parametrizadas**.
-   **Armazenamento de Senhas:** Senhas são armazenadas como **hashes criptográficos fortes** (SHA-256).
-   **Menor Privilégio:** O usuário da aplicação no banco de dados possui apenas as permissões mínimas necessárias.
