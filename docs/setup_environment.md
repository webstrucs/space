# Configuração do Ambiente de Desenvolvimento e Produção do Space Web Server

Este documento detalha o processo de configuração inicial do ambiente para o **Space Web Server**, tanto para desenvolvimento quanto para o ambiente de produção em uma VPS. As instruções são baseadas no sistema operacional Debian 12 (Bookworm).

## 1. Provisionamento da Instância VPS

O **Space Web Server** foi projetado para rodar em um ambiente Linux. Para este projeto, utilizamos uma Virtual Private Server (VPS) com as seguintes especificações mínimas:

* **Provedora:** (Mencione aqui a provedora que você utilizou, ex: DigitalOcean, Linode, AWS Lightsail, Vultr, etc.)
* **Sistema Operacional:** Debian 12 (Bookworm) 64-bit
* **Recursos de Hardware:**
    * **CPU:** 1 vCPU
    * **RAM:** 1 GB
    * **Armazenamento:** SSD de 20 GB

### Passos de Provisionamento:

1.  **Escolha da Imagem:** Ao criar a VPS, selecione a imagem oficial do **Debian 12 (Bookworm)**.
2.  **Plano de Hardware:** Opte pelo plano que atenda ou exceda as especificações mínimas acima.
3.  **Localização:** Selecione a localização do data center mais próxima aos seus usuários ou preferencial.

## 2. Configuração de Acesso SSH Seguro

O acesso seguro à VPS é fundamental. A configuração com chaves SSH aumenta significativamente a segurança em comparação com senhas.

### 2.1. Gerar Chaves SSH (Se necessário)

Se você ainda não possui um par de chaves SSH no seu computador local:

```bash
ssh-keygen -t rsa -b 4096 -C "seu_email@example.com"

---

**Próximos passos para você:**

1.  **Crie o arquivo:** No seu repositório local, crie o diretório `docs/` se ele ainda não existir e salve o conteúdo acima como `docs/setup_environment.md`.
2.  **Adicione e comite:**
    ```bash
    git add docs/setup_environment.md
    git commit -m "docs(environment): Document initial VPS setup and dependencies for #1"
    ```
3.  **Envie para o remoto:**
    ```bash
    git push origin feature/issue-001-env-setup
    ```
4.  **Crie o Pull Request:** Vá ao GitHub e crie um Pull Request da `feature/issue-001-env-setup` para a `dev`.

Quando o PR estiver pronto ou se tiver qualquer dúvida durante o processo, me avise!