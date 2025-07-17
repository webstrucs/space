#!/bin/bash

# ==============================================================================
# Script de Instalação e Configuração do Servidor Space v1.1
# ==============================================================================
# Executar como root (ou com sudo) em um sistema Debian 12 limpo.

# --- Variáveis de Configuração ---
APP_USER="spaceuser"
APP_GROUP="spacegroup"
APP_ROOT="/var/www/space"
CONFIG_DIR="/etc/space"
LOG_DIR="/var/log/space"
WORKS_DIR="/srv/space/works"
REPO_URL="https://github.com/webstrucs/space.git"

# --- CORREÇÃO: Variável para a branch a ser instalada ---
# Mude para "main" quando for fazer o deploy da versão de produção.
GIT_BRANCH="dev"

# --- Etapa 0: Verificação de Segurança ---
if [ "$(id -u)" -ne 0 ]; then
   echo "ERRO: Este script precisa ser executado como root ou com sudo." 
   exit 1
fi

echo "--- Iniciando a Instalação do Servidor Space (Branch: $GIT_BRANCH) ---"

# --- Etapa 1: Atualização e Instalação de Dependências ---
echo "--> [1/7] Atualizando o sistema e instalando dependências..."
apt-get update && apt-get upgrade -y
apt-get install -y git curl build-essential python3-venv

# --- Etapa 2: Criação do Usuário e Estrutura de Diretórios ---
echo "--> [2/7] Configurando usuário e diretórios de produção..."
if ! getent group "$APP_GROUP" >/dev/null; then
    groupadd --system "$APP_GROUP"
fi
if ! id "$APP_USER" >/dev/null 2>&1; then
    useradd --system --gid "$APP_GROUP" --home-dir "$APP_ROOT" --shell /bin/false "$APP_USER"
fi

mkdir -p "$APP_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$WORKS_DIR"

# --- Etapa 3: Deploy e Build da Aplicação ---
echo "--> [3/7] Clonando o repositório do GitHub (branch: $GIT_BRANCH)..."
# CORREÇÃO: Usamos a flag --branch para clonar a branch de desenvolvimento.
git clone --branch "$GIT_BRANCH" "$REPO_URL" "$APP_ROOT"

# --- Etapa 4: Compilação do Núcleo Rust ---
echo "--> [4/7] Instalando Rust e compilando o núcleo..."
if ! command -v cargo &> /dev/null
then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
cd "$APP_ROOT/rs_core"
cargo build --release --locked --bin server
mv target/release/server "$APP_ROOT/space_server_rust"
cd "$APP_ROOT"

# --- Etapa 5: Configuração do Ambiente Python ---
echo "--> [5/7] Configurando o ambiente virtual Python..."
python3 -m venv venv

# --- Etapa 6: Permissões e Configuração Final ---
echo "--> [6/7] Ajustando permissões e copiando arquivos de serviço..."
cp "$APP_ROOT/deploy/space-python.service" /etc/systemd/system/
cp "$APP_ROOT/deploy/space-rust.service" /etc/systemd/system/
chown -R "$APP_USER":"$APP_GROUP" "$APP_ROOT"
chown -R "$APP_USER":"$APP_GROUP" "$CONFIG_DIR"
chown -R "$APP_USER":"$APP_GROUP" "$LOG_DIR"
chown -R "$APP_USER":"$APP_GROUP" "$WORKS_DIR"
chmod +x "$APP_ROOT/space_server_rust"

# --- Etapa 7: Ativação dos Serviços ---
echo "--> [7/7] Ativando os serviços do systemd..."
systemctl daemon-reload
systemctl enable space-python.service
systemctl enable space-rust.service

echo "----------------------------------------------------"
echo "✅ Instalação da estrutura do Servidor Space (dev) concluída!"
echo ""
echo "PRÓXIMOS PASSOS MANUAIS:"
echo "1. Aponte seu domínio/subdomínio de testes para o IP desta VPS."
echo "2. Obtenha certificados SSL com 'sudo certbot certonly --standalone ...'"
echo "3. Crie e preencha o arquivo '/etc/space/production.env'."
echo "4. Inicie os serviços com: 'sudo systemctl start space-python space-rust'"
echo "----------------------------------------------------"