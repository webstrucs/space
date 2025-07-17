# Modelo para /etc/space/production.env
# COPIE ESTE ARQUIVO PARA /etc/space/production.env E AJUSTE OS VALORES

# Porta segura padrão. Requer CAP_NET_BIND_SERVICE
PORT=443

# Ajuste para o número de vCPUs da sua VPS
WORKERS=2

# Caminhos para os certificados Let's Encrypt (ajuste o domínio)
CERT_PATH=/etc/letsencrypt/live/webstrucs.com/fullchain.pem
KEY_PATH=/etc/letsencrypt/live/webstrucs.com/privkey.pem

# GERE UM NOVO SEGREDO FORTE PARA JWT!
# Use `openssl rand -hex 32` para gerar um.
JWT_SECRET=mude_este_segredo_para_algo_forte_e_aleatorio