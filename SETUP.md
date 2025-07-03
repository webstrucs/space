# Documento de Setup do Ambiente - Servidor Space

Este documento descreve o processo para configurar um ambiente de desenvolvimento e produção para o servidor Space, conforme a **Issue #001**.

## 1. Requisitos Mínimos do Servidor (VPS)

- **Sistema Operacional:** Debian 12 (Bookworm) 64-bit
- **CPU:** 1 vCPU
- **RAM:** 1 GB
- **Armazenamento:** 20 GB SSD

## 2. Procedimento de Instalação Inicial

Após provisionar a VPS com os requisitos acima e adicionar sua chave SSH, acesse o servidor e execute o script abaixo para atualizar o sistema e instalar todas as dependências essenciais.

```bash
#!/bin/bash

# --- Atualiza o sistema ---
apt update && apt upgrade -y

# --- Instala ferramentas essenciais e de compilação ---
# git: controle de versão
# curl/wget: ferramentas de download
# build-essential: compiladores C/C++ (necessário para Rust e algumas libs Python)
# python3.11-venv: para criar ambientes virtuais Python
# python3-dev: headers de desenvolvimento para Python
apt install -y git curl wget build-essential python3.11-venv python3-dev

# --- Instala o Rustup (gerenciador da toolchain Rust) ---
# Executa o script de instalação oficial do Rust.
# A instalação será feita com as opções padrão.
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh -s -- -y

# Adiciona os binários do Rust ao PATH da sessão atual
source "$HOME/.cargo/env"

echo "----------------------------------------------------"
echo "Ambiente base configurado com sucesso!"
echo "Lembre-se de configurar o acesso SSH seguro (desabilitando senhas)."
echo "Execute 'source \"\$HOME/.cargo/env\"' ou reinicie o shell para usar 'cargo'."
echo "----------------------------------------------------"
```

## 3. Configuração de Segurança SSH

Para garantir a segurança do servidor, o acesso via SSH deve ser configurado para aceitar apenas chaves públicas, desabilitando o login por senha e o acesso do usuário `root`.

1.  **Criar um novo usuário** com privilégios `sudo` e copiar a chave SSH para ele.
2.  Editar o arquivo `/etc/ssh/sshd_config` e garantir que as seguintes configurações estejam aplicadas:
    ```
    PermitRootLogin no
    PubkeyAuthentication yes
    PasswordAuthentication no
    PermitEmptyPasswords no
    ```
3.  Reiniciar o serviço SSH com `sudo systemctl restart sshd`.

**Aviso:** Sempre teste o login com a chave SSH em um novo terminal antes de fechar a sessão atual para evitar perder o acesso.