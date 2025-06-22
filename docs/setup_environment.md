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