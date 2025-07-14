#!/bin/bash
# entrypoint.sh

# Garante que o script pare se algum comando falhar
set -e

# Inicia o servidor IPC em Python em segundo plano
echo "Iniciando o servidor Python (IPC)..."
python3 main_server.py &

# Aguarda um segundo para garantir que o socket IPC seja criado
sleep 1

# Inicia o servidor principal em Rust em primeiro plano
# O processo do contêiner ficará atrelado a este.
echo "Iniciando o servidor Rust (Gateway)..."
./server