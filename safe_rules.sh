#!/bin/bash

# Script interativo para configuração de segurança de um servidor Debian 12
# Configura nftables, altera porta SSH, protege serviços da pilha TCP/IP e implementa proteções contra ataques

# Verifica se o script está sendo executado como root
if [[ $EUID -ne 0 ]]; then
    echo "Este script precisa ser executado como root. Use sudo."
    exit 1
fi

# Variáveis globais
ssh_port=2222
services=""

# Função para gerar um backup do sistema antes de alterações
backup_system() {
    echo "Criando backup das configurações atuais..."
    mkdir -p /root/backup
    cp /etc/ssh/sshd_config /root/backup/sshd_config.bak 2>/dev/null
    cp /etc/nftables.conf /root/backup/nftables.conf.bak 2>/dev/null
    echo "Backup concluído em /root/backup."
}

# Função para instalar pacotes necessários
install_packages() {
    echo "Atualizando sistema e instalando pacotes necessários..."
    apt update && apt upgrade -y
    apt install -y nftables fail2ban
}

# Função para configurar a nova porta SSH
configure_ssh_port() {
    echo "A porta padrão SSH (22) será alterada para aumentar a segurança."
    echo "Escolha uma nova porta (1024-65535, recomendado: 2222, 2244, 2255):"
    read -p "Digite a nova porta SSH [2222]: " user_port
    ssh_port=${user_port:-2222}

    # Valida a porta
    if ! [[ "$ssh_port" =~ ^[0-9]+$ ]] || [ "$ssh_port" -lt 1024 ] || [ "$ssh_port" -gt 65535 ]; then
        echo "Porta inválida. Usando porta padrão 2222."
        ssh_port=2222
    fi

    # Altera a porta no sshd_config
    sed -i "s/#Port 22/Port $ssh_port/" /etc/ssh/sshd_config
    sed -i "s/Port 22/Port $ssh_port/" /etc/ssh/sshd_config
    echo "Porta SSH alterada para $ssh_port."
}

# Função para configurar regras de nftables para serviços da pilha TCP/IP
configure_nftables() {
    echo "Configurando regras de firewall com nftables..."
    echo "Quais serviços/protocols da pilha TCP/IP você deseja permitir no servidor? (Digite os números, separados por espaço)"
    echo "1) HTTP (TCP, porta 80)"
    echo "2) HTTPS (TCP, porta 443, TLS)"
    echo "3) SSH (TCP, porta $ssh_port)"
    echo "4) FTP (TCP, portas 20-21, modo ativo/passivo)"
    echo "5) FTPS (TCP, portas 20-21, TLS)"
    echo "6) SFTP (TCP, porta $ssh_port)"
    echo "7) DNS (UDP/TCP, porta 53)"
    echo "8) SMTP (TCP, porta 25)"
    echo "9) SMTPS (TCP, porta 465, TLS)"
    echo "10) IMAP (TCP, porta 143)"
    echo "11) IMAPS (TCP, porta 993, TLS)"
    echo "12) POP3 (TCP, porta 110)"
    echo "13) POP3S (TCP, porta 995, TLS)"
    echo "14) QUIC (UDP, porta 443)"
    echo "15) SIP (UDP/TCP, portas 5060-5061)"
    echo "16) RTP/RTSP (UDP, portas 10000-20000)"
    echo "17) WebSocket (WS, TCP, porta 80)"
    echo "18) WebSocket Secure (WSS, TCP, porta 443, TLS)"
    echo "19) Outros (especifique as portas e protocolo TCP/UDP)"
    read -p "Digite as opções (ex: 1 2 3): " services

    # Cria arquivo de configuração do nftables
    cat > /etc/nftables.conf << 'EOF'
#!/usr/sbin/nft -f

flush ruleset

table inet filter {
    chain input {
        type filter hook input priority 0; policy drop;

        # Permitir tráfego loopback
        iif lo accept

        # Permitir tráfego estabelecido e relacionado
        ct state established,related accept

        # Proteger contra varreduras de portas
        ct state invalid drop
        tcp flags & (fin|syn|rst|ack) != syn ct state new drop

        # Limitar conexões novas para evitar ataques de flood
        ct state new limit rate 10/second accept

        # Permitir ICMP (ping) com limite
        ip protocol icmp icmp type echo-request limit rate 5/second accept
        ip6 nexthdr icmpv6 icmpv6 type echo-request limit rate 5/second accept
EOF

    # Adiciona regras com base nas escolhas do usuário
    for service in $services; do
        case $service in
            1)
                echo "        # Permitir HTTP (TCP)" >> /etc/nftables.conf
                echo "        tcp dport 80 limit rate 50/second accept" >> /etc/nftables.conf
                ;;
            2)
                echo "        # Permitir HTTPS (TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 443 limit rate 50/second accept" >> /etc/nftables.conf
                ;;
            3)
                echo "        # Permitir SSH (TCP)" >> /etc/nftables.conf
                echo "        tcp dport $ssh_port limit rate 5/second accept" >> /etc/nftables.conf
                ;;
            4)
                echo "        # Permitir FTP (TCP, controle e dados)" >> /etc/nftables.conf
                echo "        tcp dport 21 limit rate 10/second accept" >> /etc/nftables.conf
                echo "        tcp dport 20 limit rate 10/second accept" >> /etc/nftables.conf
                echo "        tcp dport 40000-50000 ct state established,related accept" >> /etc/nftables.conf
                ;;
            5)
                echo "        # Permitir FTPS (TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 21 limit rate 10/second accept" >> /etc/nftables.conf
                echo "        tcp dport 20 limit rate 10/second accept" >> /etc/nftables.conf
                echo "        tcp dport 40000-50000 ct state established,related accept" >> /etc/nftables.conf
                ;;
            6)
                echo "        # Permitir SFTP (TCP, usa mesma porta SSH)" >> /etc/nftables.conf
                echo "        tcp dport $ssh_port limit rate 5/second accept" >> /etc/nftables.conf
                ;;
            7)
                echo "        # Permitir DNS (UDP/TCP)" >> /etc/nftables.conf
                echo "        tcp dport 53 limit rate 100/second accept" >> /etc/nftables.conf
                echo "        udp dport 53 limit rate 100/second accept" >> /etc/nftables.conf
                ;;
            8)
                echo "        # Permitir SMTP (TCP)" >> /etc/nftables.conf
                echo "        tcp dport 25 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            9)
                echo "        # Permitir SMTPS (TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 465 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            10)
                echo "        # Permitir IMAP (TCP)" >> /etc/nftables.conf
                echo "        tcp dport 143 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            11)
                echo "        # Permitir IMAPS (TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 993 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            12)
                echo "        # Permitir POP3 (TCP)" >> /etc/nftables.conf
                echo "        tcp dport 110 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            13)
                echo "        # Permitir POP3S (TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 995 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            14)
                echo "        # Permitir QUIC (UDP)" >> /etc/nftables.conf
                echo "        udp dport 443 limit rate 50/second accept" >> /etc/nftables.conf
                ;;
            15)
                echo "        # Permitir SIP (UDP/TCP)" >> /etc/nftables.conf
                echo "        tcp dport 5060-5061 limit rate 20/second accept" >> /etc/nftables.conf
                echo "        udp dport 5060-5061 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            16)
                echo "        # Permitir RTP/RTSP (UDP)" >> /etc/nftables.conf
                echo "        udp dport 10000-20000 limit rate 100/second accept" >> /etc/nftables.conf
                echo "        tcp dport 554 limit rate 20/second accept" >> /etc/nftables.conf
                ;;
            17)
                echo "        # Permitir WebSocket (WS, TCP)" >> /etc/nftables.conf
                echo "        tcp dport 80 limit rate 50/second accept" >> /etc/nftables.conf
                ;;
            18)
                echo "        # Permitir WebSocket Secure (WSS, TCP, TLS)" >> /etc/nftables.conf
                echo "        tcp dport 443 limit rate 50/second accept" >> /etc/nftables.conf
                ;;
            19)
                read -p "Digite as portas e protocolo (ex: 8080/tcp 8443/udp): " custom_ports
                for port_proto in $custom_ports; do
                    port=$(echo $port_proto | cut -d'/' -f1)
                    proto=$(echo $port_proto | cut -d'/' -f2)
                    if [[ "$port" =~ ^[0-9]+$ ]] && [ "$port" -ge 1 ] && [ "$port" -le 65535 ] && [[ "$proto" =~ ^(tcp|udp)$ ]]; then
                        echo "        # Permitir porta personalizada $port ($proto)" >> /etc/nftables.conf
                        echo "        $proto dport $port limit rate 20/second accept" >> /etc/nftables.conf
                    else
                        echo "Porta ou protocolo $port_proto inválido, ignorado."
                    fi
                done
                ;;
            *)
                echo "Opção inválida: $service"
                ;;
        esac
    done

    # Finaliza a configuração do nftables
    cat >> /etc/nftables.conf << 'EOF'
        # Bloquear todo o resto
        reject with icmpx type port-unreachable
    }

    chain forward {
        type filter hook forward priority 0; policy drop;
    }

    chain output {
        type filter hook output priority 0; policy accept;
    }
}
EOF

    # Ativa e carrega as regras
    systemctl enable nftables
    nft -f /etc/nftables.conf
    systemctl restart nftables
    echo "Regras de nftables configuradas com suporte à pilha TCP/IP."
}

# Função para configurar Fail2Ban com suporte a serviços da camada de aplicação
configure_fail2ban() {
    echo "Configurando Fail2Ban para proteção contra força bruta..."
    
    # Criar arquivo jail.local com substituição de variável
    cat > /etc/fail2ban/jail.local << EOF
[DEFAULT]
bantime  = 3600
findtime = 600
maxretry = 5

[sshd]
enabled   = true
port      = $ssh_port
filter    = sshd
logpath   = /var/log/auth.log
maxretry  = 3
EOF

    # Adiciona configurações para serviços da camada de aplicação
    for service in $services; do
        case $service in
            4|5)
                echo -e "\n[vsftpd]\nenabled = true\nport = 20,21,40000-50000\nfilter = vsftpd\nlogpath = /var/log/vsftpd.log\nmaxretry = 5" >> /etc/fail2ban/jail.local
                ;;
            6)
                echo -e "\n[sftp]\nenabled = true\nport = $ssh_port\nfilter = sshd\nlogpath = /var/log/auth.log\nmaxretry = 3" >> /etc/fail2ban/jail.local
                ;;
            8|9)
                echo -e "\n[postfix]\nenabled = true\nport = 25,465\nfilter = postfix\nlogpath = /var/log/mail.log\nmaxretry = 5" >> /etc/fail2ban/jail.local
                ;;
            10|11|12|13)
                echo -e "\n[dovecot]\nenabled = true\nport = 143,993,110,995\nfilter = dovecot\nlogpath = /var/log/mail.log\nmaxretry = 5" >> /etc/fail2ban/jail.local
                ;;
            15)
                echo -e "\n[asterisk]\nenabled = true\nport = 5060,5061\nfilter = asterisk\nlogpath = /var/log/asterisk/messages\nmaxretry = 5" >> /etc/fail2ban/jail.local
                ;;
        esac
    done

    systemctl enable fail2ban
    systemctl restart fail2ban
    echo "Fail2Ban configurado com suporte a serviços da pilha TCP/IP."
}

# Função para configurações adicionais de segurança
additional_security() {
    echo "Deseja aplicar configurações adicionais de segurança? (y/n)"
    echo "1) Configurar atualizações automáticas de segurança"
    echo "2) Proteger contra ataques DDoS na camada de aplicação"
    echo "3) Habilitar proteção contra spoofing de IP"
    echo "4) Configurar hardening adicional do SSH"
    read -p "Digite y para configurar, n para pular [n]: " apply_extra
    apply_extra=${apply_extra:-n}

    if [[ "$apply_extra" == "y" ]]; then
        # Atualizações automáticas
        read -p "Configurar atualizações automáticas de segurança? (y/n) [y]: " auto_updates
        auto_updates=${auto_updates:-y}
        if [[ "$auto_updates" == "y" ]]; then
            apt install -y unattended-upgrades
            echo 'Unattended-Upgrade::Automatic-Reboot "false";' >> /etc/apt/apt.conf.d/50unattended-upgrades
            systemctl enable unattended-upgrades
            echo "Atualizações automáticas configuradas."
        fi

        # Proteção contra DDoS na camada de aplicação
        read -p "Configurar proteção contra DDoS na camada de aplicação? (y/n) [y]: " ddos_protection
        ddos_protection=${ddos_protection:-y}
        if [[ "$ddos_protection" == "y" ]]; then
            echo "Configurando proteção contra DDoS..."
            # Adiciona regras de rate limiting mais rigorosas
            sed -i '/ct state new limit rate 10\/second accept/c\        ct state new limit rate 10/second burst 15 packets accept' /etc/nftables.conf
            nft -f /etc/nftables.conf
            echo "Proteção contra DDoS configurada no nftables."
        fi

        # Proteção contra spoofing de IP
        read -p "Habilitar proteção contra spoofing de IP? (y/n) [y]: " spoof_protection
        spoof_protection=${spoof_protection:-y}
        if [[ "$spoof_protection" == "y" ]]; then
            echo "Configurando proteção contra spoofing de IP..."
            echo "net.ipv4.conf.all.rp_filter = 1" >> /etc/sysctl.conf
            echo "net.ipv4.conf.default.rp_filter = 1" >> /etc/sysctl.conf
            echo "net.ipv4.icmp_echo_ignore_broadcasts = 1" >> /etc/sysctl.conf
            echo "net.ipv4.icmp_ignore_bogus_error_responses = 1" >> /etc/sysctl.conf
            sysctl -p
            echo "Proteção contra spoofing de IP habilitada."
        fi

        # Hardening adicional do SSH (sem desabilitar root ou limitar usuários)
        read -p "Aplicar hardening adicional do SSH? (y/n) [y]: " ssh_hardening
        ssh_hardening=${ssh_hardening:-y}
        if [[ "$ssh_hardening" == "y" ]]; then
            echo "Configurando hardening adicional do SSH..."
            # Desabilitar protocolos antigos e configurações inseguras
            sed -i 's/#Protocol.*/Protocol 2/' /etc/ssh/sshd_config
            sed -i 's/#MaxAuthTries.*/MaxAuthTries 3/' /etc/ssh/sshd_config
            sed -i 's/#ClientAliveInterval.*/ClientAliveInterval 300/' /etc/ssh/sshd_config
            sed -i 's/#ClientAliveCountMax.*/ClientAliveCountMax 2/' /etc/ssh/sshd_config
            sed -i 's/#LoginGraceTime.*/LoginGraceTime 60/' /etc/ssh/sshd_config
            sed -i 's/#X11Forwarding.*/X11Forwarding no/' /etc/ssh/sshd_config
            sed -i 's/#PermitEmptyPasswords.*/PermitEmptyPasswords no/' /etc/ssh/sshd_config
            echo "Hardening adicional do SSH aplicado."
        fi
    fi
}

# Função para reiniciar serviços
restart_services() {
    echo "Reiniciando serviços..."
    systemctl restart sshd
    systemctl restart nftables
    systemctl restart fail2ban
    echo "Serviços reiniciados."
}

# Função principal
main() {
    echo "Bem-vindo ao script de configuração de segurança para Debian 12!"
    echo "Este script configurará o firewall (nftables), SSH, Fail2Ban e proteções para a pilha TCP/IP."
    
    backup_system
    install_packages
    configure_ssh_port
    configure_nftables
    configure_fail2ban
    additional_security
    restart_services

    echo "Configuração concluída!"
    echo "Resumo:"
    echo "- Porta SSH: $ssh_port"
    echo "- Serviços permitidos: $services"
    echo "- Fail2Ban configurado para SSH e serviços da pilha TCP/IP"
    echo "- Backup salvo em /root/backup"
    echo ""
    echo "IMPORTANTE: Teste a conexão SSH na nova porta antes de desconectar!"
    echo "Exemplo: ssh -p $ssh_port usuario@servidor"
    echo ""
    echo "Verifique as configurações em /etc/nftables.conf e /etc/ssh/sshd_config."
}

# Executa o script
main