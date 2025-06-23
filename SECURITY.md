# Políticas de Segurança de Baixo Nível - Space Web Server (rust_core)

Este documento detalha as medidas de segurança implementadas na camada de baixo nível do Space Web Server, especificamente no módulo `rust_core/space_core`.

## 1. Prevenção de Negação de Serviço (DoS) e Gerenciamento de Recursos

### 1.1. Rate Limiting de Novas Conexões
* **Mecanismo:** Um limite de taxa global é imposto a novas conexões TCP/IP.
* **Configuração Atual:** O servidor é configurado para aceitar **até 100.000 novas conexões por segundo**, com uma capacidade de estouro (burst) também de 100.000 conexões adicionais para picos de tráfego. Novas conexões que excedam este limite são recusadas.
* **Benefício:** Ajuda a mitigar ataques básicos de negação de serviço (DoS) como o SYN Flood (onde um grande número de novas conexões é tentado) e protege o servidor contra sobrecarga de recursos de conexão, permitindo, ao mesmo tempo, um alto volume de tráfego legítimo.

### 1.2. Idle Timeout para Conexões Inativas
* **Mecanismo:** Conexões TCP/IP são automaticamente encerradas se permanecerem inativas (sem atividade de leitura de dados) por um período configurável.
* **Configuração Atual:** O tempo limite de inatividade está definido para **30 segundos**.
* **Benefício:** Libera recursos do servidor (memória, descritores de arquivo) que seriam mantidos por conexões ociosas ou "travadas", prevenindo o esgotamento de recursos e aumentando a disponibilidade.

### 1.3. TCP Keep-Alive
* **Mecanismo:** A opção `SO_KEEPALIVE` é configurada nos sockets TCP, permitindo ao sistema operacional enviar pacotes de `keep-alive` periodicamente em conexões ociosas.
* **Configuração Atual:**
    * **`time` (tempo de inatividade inicial):** 60 segundos antes de enviar o primeiro probe.
    * **`interval` (intervalo entre probes):** 10 segundos entre probes subsequentes.
    * **`retries` (tentativas):** 3 probes antes de considerar a conexão morta.
* **Benefício:** Ajuda a detectar e encerrar conexões com clientes que se desconectaram abruptamente (por exemplo, queda de rede, reinicialização do cliente) sem enviar um fechamento gracioso (FIN/RST), prevenindo conexões "órfãs" e liberando recursos.

## 2. Prevenção de Buffer Overflows

### 2.1. Uso Seguro de Buffers
* **Mecanismo:** A leitura de dados da rede é feita em buffers de tamanho fixo (`Vec<u8>`). A linguagem Rust, através de sua segurança de memória e verificação de limites em tempo de compilação/execução, garante que as operações de leitura (como `socket.read(&mut buf)`) nunca escrevam bytes além da capacidade alocada do buffer.
* **Configuração Atual:** O buffer de leitura para cada conexão é fixado em **4096 bytes**.
* **Benefício:** Elimina a maioria dos riscos comuns de `buffer overflow`, garantindo que dados maliciosos ou excessivos não possam sobrescrever a memória adjacente, prevenindo vulnerabilidades como execução de código arbitrário. Se a entrada exceder o tamanho do buffer, ela será lida em múltiplos pedaços de forma segura.

## 3. Outras Considerações de Segurança (Futuras/Externas)

* **Sanitização de Entradas:** A sanitização de entradas de dados HTTP (após o parsing) será tratada na camada de protocolo/aplicação.
* **Isolamento de Processos:** Técnicas como `chroot` ou contêineres para isolamento de processos são consideradas para o ambiente de deploy/produção e não são implementadas no código Rust do servidor.
* **Detecção de Anomalias no Tráfego:** Atualmente, a detecção é rudimentar (baseada em rate limiting e timeouts). Ferramentas de logging e monitoramento mais avançadas serão exploradas em futuras iterações para identificar padrões de tráfego maliciosos.

---
**Nota:** Esta documentação será atualizada conforme novas políticas e implementações de segurança forem introduzidas.