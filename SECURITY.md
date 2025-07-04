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