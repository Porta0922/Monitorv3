#!/bin/bash
# ActivityMonitor Enterprise v3 - Linux Installer
# Creates systemd service unit for agent

set -e

# Configuration
AGENT_NAME="activity-monitor-agent"
SERVICE_NAME="activity-monitor-agent"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_BIN="$SCRIPT_DIR/../target/release/$AGENT_NAME"
AGENT_PATH="/opt/activity-monitor/bin/$AGENT_NAME"
CONFIG_DIR="/etc/activity-monitor"
LOG_DIR="/var/log/activity-monitor"
DATA_DIR="/var/lib/activity-monitor"
ENV_FILE="$CONFIG_DIR/.env"
NICKNAME_FILE="$DATA_DIR/device_nickname.txt"

# Default Variables
AGENT_AUTH_TOKEN="dev-agent-token"
AGENT_SERVER_URL="http://localhost:3000"
RABBITMQ_URL="amqp://guest:guest@127.0.0.1:5672/%2f"
AGENT_OFFLINE_CACHE_KEY="replace-with-32-byte-cache-key!!"

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
   echo "[-] This script must be run as root. (Use: sudo ./install-linux.sh)"
   exit 1
fi

# Load existing environment variables if present
if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
fi

show_menu() {
    clear
    echo "========================================"
    echo "ActivityMonitor Enterprise v3 Installer (Linux)"
    echo "========================================"
    echo "Rutas de instalacion:"
    echo "- Ejecutable: $AGENT_PATH"
    echo "- Configuracion: $CONFIG_DIR"
    echo "- Logs: $LOG_DIR"
    echo "========================================"
    echo ""
    echo "Seleccione una opcion:"
    echo "1. Instalar (o actualizar) el agente"
    echo "2. Modificar credenciales (.env) y reiniciar"
    echo "3. Desinstalar"
    echo "4. Salir"
    echo ""
    read -p "Opcion: " MENU_OPTION

    case $MENU_OPTION in
        1) install_agent ;;
        2) modify_creds ;;
        3) uninstall_agent ;;
        4) exit 0 ;;
        *) show_menu ;;
    esac
}

modify_creds() {
    echo ""
    echo "=== Modificar Credenciales ==="
    read -p "Enter agent auth token (default: $AGENT_AUTH_TOKEN): " INPUT_AUTH_TOKEN
    [ -n "$INPUT_AUTH_TOKEN" ] && AGENT_AUTH_TOKEN=$INPUT_AUTH_TOKEN

    read -p "Enter server URL (default: $AGENT_SERVER_URL): " INPUT_SERVER_URL
    [ -n "$INPUT_SERVER_URL" ] && AGENT_SERVER_URL=$INPUT_SERVER_URL

    read -p "Enter RabbitMQ URL (default: $RABBITMQ_URL): " INPUT_RABBITMQ_URL
    [ -n "$INPUT_RABBITMQ_URL" ] && RABBITMQ_URL=$INPUT_RABBITMQ_URL

    mkdir -p "$CONFIG_DIR"
    echo "[*] Guardando configuracion..."
    cat > "$ENV_FILE" << ENVEOF
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
    chmod 600 "$ENV_FILE"
    echo "[+] Credenciales actualizadas en: $ENV_FILE"

    if systemctl is-active --quiet $SERVICE_NAME.service; then
        echo "[*] Reiniciando servicio para aplicar cambios..."
        systemctl restart $SERVICE_NAME.service
        echo "[+] Servicio reiniciado."
    else
        echo "[!] El servicio no esta en ejecucion. Instale primero."
    fi

    echo ""
    read -n 1 -s -r -p "Presione cualquier tecla para volver al menu..."
    show_menu
}

uninstall_agent() {
    echo ""
    echo "=== Desinstalando ==="
    if systemctl is-active --quiet $SERVICE_NAME.service || systemctl is-failed --quiet $SERVICE_NAME.service; then
        echo "[*] Deteniendo servicio..."
        systemctl stop $SERVICE_NAME.service || true
        echo "[*] Deshabilitando servicio..."
        systemctl disable $SERVICE_NAME.service || true
        
        echo "[*] Removiendo unidad de systemd..."
        rm -f /etc/systemd/system/$SERVICE_NAME.service
        systemctl daemon-reload
    else
        echo "[!] El servicio no esta instalado o registrado en systemd."
    fi

    echo "[*] Deteniendo agente en ejecucion (si existe)..."
    pkill -f "$AGENT_NAME" || true

    echo "[+] Desinstalacion completada."
    echo ""
    read -n 1 -s -r -p "Presione cualquier tecla para volver al menu..."
    show_menu
}

install_agent() {
    echo ""
    echo "=== Instalando Agente ==="
    
    # Prompt for credentials
    read -p "Enter agent auth token (default: $AGENT_AUTH_TOKEN): " INPUT_AUTH_TOKEN
    [ -n "$INPUT_AUTH_TOKEN" ] && AGENT_AUTH_TOKEN=$INPUT_AUTH_TOKEN

    read -p "Enter server URL (default: $AGENT_SERVER_URL): " INPUT_SERVER_URL
    [ -n "$INPUT_SERVER_URL" ] && AGENT_SERVER_URL=$INPUT_SERVER_URL

    read -p "Enter RabbitMQ URL (default: $RABBITMQ_URL): " INPUT_RABBITMQ_URL
    [ -n "$INPUT_RABBITMQ_URL" ] && RABBITMQ_URL=$INPUT_RABBITMQ_URL

    # Optional nickname prompt
    if [ ! -f "$NICKNAME_FILE" ]; then
        read -p "Enter device nickname (or press Enter for hostname): " DEVICE_NICKNAME
        if [ -z "$DEVICE_NICKNAME" ]; then
            DEVICE_NICKNAME=$(hostname)
        fi
    fi

    echo ""
    echo "[Paso 1/4] Preparando directorios y configuracion..."
    mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    mkdir -p /opt/activity-monitor/bin

    cat > "$ENV_FILE" << ENVEOF
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
    chmod 600 "$ENV_FILE"

    if [ -n "$DEVICE_NICKNAME" ]; then
        echo "$DEVICE_NICKNAME" > "$NICKNAME_FILE"
    fi

    echo "[Paso 2/4] Verificando/Compilando el agente..."
    CARGO_FOUND=0
    if command -v cargo &> /dev/null; then
        CARGO_FOUND=1
    elif [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
        command -v cargo &> /dev/null && CARGO_FOUND=1
    elif [ -f "/root/.cargo/env" ]; then
        source "/root/.cargo/env"
        command -v cargo &> /dev/null && CARGO_FOUND=1
    fi

    if [ $CARGO_FOUND -eq 1 ]; then
        echo "[*] cargo encontrado. Compilando la ultima version del agente..."
        pushd "$SCRIPT_DIR/.." > /dev/null
        cargo build --release -p activity-monitor-agent
        if [ $? -ne 0 ]; then
            echo "[-] Fallo en la compilacion del agente."
            popd > /dev/null
            exit 1
        fi
        popd > /dev/null
    else
        echo "[*] cargo no encontrado. Verificando binario pre-compilado..."
    fi

    if [ ! -f "$TARGET_BIN" ]; then
        echo "[-] Error: cargo (Rust) no esta instalado y no se encontro binario en $TARGET_BIN"
        echo "[-] Instale Rust toolchain primero o copie un ejecutable pre-compilado."
        exit 1
    else
        echo "[+] Usando binario en $TARGET_BIN"
    fi

    echo "[*] Deteniendo servicio si esta en ejecucion..."
    systemctl stop $SERVICE_NAME.service 2>/dev/null || true

    echo "[*] Copiando binario a $AGENT_PATH..."
    cp -f "$TARGET_BIN" "$AGENT_PATH"
    chmod 755 "$AGENT_PATH"

    echo ""
    echo "[Paso 3/4] Creando usuario y permisos..."
    if ! id -u activity-monitor &>/dev/null; then
        useradd --system --home $DATA_DIR --shell /usr/sbin/nologin activity-monitor
    fi

    chown -R root:root "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    chmod 750 "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    chmod 640 "$NICKNAME_FILE" 2>/dev/null || true

    echo ""
    echo "[Paso 4/4] Configurando e iniciando servicio (systemd)..."
    cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=ActivityMonitor Enterprise Agent
Documentation=https://github.com/yourrepo/ActivityMonitor-Enterprise-v3
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
# We must run as root to access /dev/input for keystroke capture natively.
WorkingDirectory=$DATA_DIR
StandardOutput=journal
StandardError=journal
SyslogIdentifier=$SERVICE_NAME

# Restart policy
Restart=on-failure
RestartSec=10s

# Resource limits
MemoryLimit=256M
CPUQuota=50%

# Start command
EnvironmentFile=-$ENV_FILE
ExecStart=$AGENT_PATH

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable $SERVICE_NAME.service
    systemctl start $SERVICE_NAME.service

    if systemctl is-active --quiet $SERVICE_NAME.service; then
        echo "[+] Servicio instalado y en ejecucion correctamente!"
    else
        echo "[-] Error al iniciar el servicio. Revisa los logs con: journalctl -u $SERVICE_NAME.service -n 20"
        exit 1
    fi

    echo ""
    echo "========================================"
    echo "Instalacion Completada"
    echo "========================================"
    echo "Resumen de configuracion:"
    echo "- API Remota: $AGENT_SERVER_URL"
    echo "- RabbitMQ:   $RABBITMQ_URL"
    echo "- Token:      $AGENT_AUTH_TOKEN"
    echo ""
    echo "Rutas:"
    echo "- Servicio:   $SERVICE_NAME"
    echo "- Ejecutable: $AGENT_PATH"
    echo "- Config:     $CONFIG_DIR"
    echo "- Logs:       $LOG_DIR (via journalctl)"
    echo ""
    echo "Para gestionar el servicio:"
    echo "  Start:   systemctl start $SERVICE_NAME"
    echo "  Stop:    systemctl stop $SERVICE_NAME"
    echo "  Logs:    journalctl -u $SERVICE_NAME -f"
    echo "========================================"
    
    echo ""
    read -n 1 -s -r -p "Presione cualquier tecla para volver al menu..."
    show_menu
}

# Iniciar Menu
show_menu
