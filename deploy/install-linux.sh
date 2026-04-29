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
AGENT_AUTH_TOKEN="change-me-in-production"
AGENT_SERVER_URL="http://10.30.0.123:3000"
RABBITMQ_URL="amqp://eclub:eCLUB123@10.30.0.123:5672/%2f"
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
    echo "ActivityMonitor Enterprise v3.1.0-HYBRID (Linux)"
    echo "========================================"
    echo "Rutas de instalacion:"
    echo "- Ejecutable: $AGENT_PATH"
    echo "- Configuracion: $CONFIG_DIR"
    echo "- Logs: $LOG_DIR"
    echo "========================================"
    echo ""
    echo "Seleccione una opcion:"
    echo "1. Instalacion COMPLETA (Servicio Systemd + Autostart Usuario)"
    echo "2. Instalar solo SERVICIO (Systemd / Background)"
    echo "3. Instalar solo AUTOSTART USUARIO (Activity tracking)"
    echo "4. Modificar credenciales (.env) y reiniciar"
    echo "5. Actualizar binario (Mantiene configuracion)"
    echo "6. Desinstalar TODO"
    echo "7. Salir"
    echo ""
    read -p "Opcion: " MENU_OPTION

    case $MENU_OPTION in
        1) MODE="FULL"; install_agent ;;
        2) MODE="SERVICE"; install_agent ;;
        3) MODE="USER"; install_agent ;;
        4) modify_creds ;;
        5) update_only ;;
        6) uninstall_agent ;;
        7) exit 0 ;;
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

update_only() {
    echo ""
    echo "[*] Iniciando actualizacion rapida (manteniendo configuracion actual)..."
    STEP_BY_STEP_INSTALL=1
    execute_steps
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
    
    execute_steps
}

execute_steps() {
    echo ""
    echo "[1/6] Preparando directorios..."
    mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    mkdir -p /opt/activity-monitor/bin
    echo "    > Directorios listos [OK]"

    echo "[2/6] Configurando archivo .env..."
    if [ ! -f "$ENV_FILE" ] || [ "$MENU_OPTION" == "1" ]; then
        cat > "$ENV_FILE" << ENVEOF
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
        chmod 600 "$ENV_FILE"
        echo "    > Archivo .env configurado [OK]"
    else
        echo "    > Archivo .env ya existe - se mantiene actual [OK]"
    fi

    if [ -n "$DEVICE_NICKNAME" ]; then
        echo "$DEVICE_NICKNAME" > "$NICKNAME_FILE"
    fi

    echo "[3/6] Verificando/Compilando el agente..."
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

    if [ $CARGO_FOUND -eq 0 ]; then
        echo "[*] cargo no encontrado. Intentando instalar Rust..."
        if command -v curl &> /dev/null; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            if [ -f "$HOME/.cargo/env" ]; then
                source "$HOME/.cargo/env"
                CARGO_FOUND=1
            fi
        else
            echo "[-] curl no esta instalado. No se puede instalar Rust automaticamente."
        fi
    fi

    # Install build dependencies for OpenSSL and compilation
    echo "[*] Verificando dependencias de compilacion (OpenSSL, pkg-config, X11)..."
    if command -v apt-get &> /dev/null; then
        apt-get update -y &> /dev/null
        apt-get install -y libssl-dev pkg-config build-essential libxtst-dev libx11-dev &> /dev/null
        echo "    > Dependencias instaladas [OK]"
    elif command -v dnf &> /dev/null; then
        dnf install -y openssl-devel pkgconf-pkg-config @development-tools libXtst-devel libX11-devel &> /dev/null
        echo "    > Dependencias instaladas [OK]"
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
        echo "[*] No se pudo instalar cargo. Verificando binario pre-compilado..."
    fi

    if [ ! -f "$TARGET_BIN" ]; then
        echo "[-] Error: cargo (Rust) no esta instalado y no se encontro binario en $TARGET_BIN"
        echo "[-] Instale Rust toolchain primero o copie un ejecutable pre-compilado."
        exit 1
    else
        echo "[+] Usando binario en $TARGET_BIN"
    fi

    echo "[4/6] Desplegando binario en la ruta local..."
    systemctl stop $SERVICE_NAME.service 2>/dev/null || true
    cp -f "$TARGET_BIN" "$AGENT_PATH"
    chmod 755 "$AGENT_PATH"
    echo "    > Binario copiado a $AGENT_PATH [OK]"

    echo "[5/6] Creando usuario y permisos..."
    if ! id -u activity-monitor &>/dev/null; then
        useradd --system --home $DATA_DIR --shell /usr/sbin/nologin activity-monitor
    fi
    
    # Clean and build
    echo "    [*] Compilando agente (esto puede tardar unos minutos)..."
    # Build from the workspace root (one level up from deploy)
    if (cd .. && cargo build --release --bin activity-monitor-agent); then
        echo "    ✅ Compilacion exitosa."
    else
        echo "    ❌ ERROR: Fallo la compilacion. Revisa las dependencias."
        exit 1
    fi

    cp ../target/release/activity-monitor-agent "$AGENT_PATH"
    chmod +x "$AGENT_PATH"

    # Add the interactive user to necessary groups for activity and wifi tracking
    if [ -n "$SUDO_USER" ]; then
        echo "    [*] Configurando grupos para $SUDO_USER..."
        usermod -aG input $SUDO_USER > /dev/null 2>&1
        usermod -aG netdev $SUDO_USER > /dev/null 2>&1
    fi

    # Ensure current user is in critical groups for monitoring
    echo "    [*] Configurando permisos de hardware (input, netdev)..."
    CURRENT_USER=$(logname 2>/dev/null || echo $USER)
    usermod -aG input,netdev "$CURRENT_USER"
    echo "    ⚠️  ATENCION: Se ha añadido al usuario $CURRENT_USER a los grupos 'input' y 'netdev'."
    echo "    ⚠️  Para que el monitoreo de teclado y WiFi funcione, DEBES REINICIAR LA SESION o la MAQUINA."

    chown -R root:root "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    chmod 750 "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    echo "    > Permisos y configuracion finalizada [OK]"

    echo ""
    echo "[6/6] Configurando e iniciando servicio..."
    
    if [ "$MODE" != "USER" ]; then
        echo "    [*] Configurando servicio systemd..."
        cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=ActivityMonitor Enterprise Agent (System Service)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=$DATA_DIR
EnvironmentFile=-$ENV_FILE
ExecStart=$AGENT_PATH
Restart=on-failure
RestartSec=10s
MemoryLimit=256M
CPUQuota=50%

[Install]
WantedBy=multi-user.target
EOF
        systemctl daemon-reload
        systemctl enable $SERVICE_NAME.service
        systemctl start $SERVICE_NAME.service
        echo "    ^> Servicio systemd configurado [OK]"
    fi

    if [ "$MODE" != "SERVICE" ]; then
        echo "    [*] Configurando autostart de usuario..."
        # We look for common user home directories if running as root
        # This is a bit tricky for all users, but we can target the current non-root user if known
        # Or just provide instructions.
        USER_AUTOSTART_DIR="/home/$SUDO_USER/.config/autostart"
        if [ -d "/home/$SUDO_USER" ]; then
            mkdir -p "$USER_AUTOSTART_DIR"
            cat > "$USER_AUTOSTART_DIR/activity-monitor.desktop" << EOF
[Desktop Entry]
Type=Application
Name=ActivityMonitor Agent
Exec=$AGENT_PATH
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
Comment=Capture user activity and window titles
EOF
            chown -R $SUDO_USER:$SUDO_USER "/home/$SUDO_USER/.config"
            echo "    ^> Autostart de usuario configurado para $SUDO_USER [OK]"
        else
            echo "    [!] No se pudo encontrar el directorio de autostart para $SUDO_USER"
        fi
    fi

    if [ "$MODE" != "USER" ] && systemctl is-active --quiet $SERVICE_NAME.service; then
        echo "[+] Servicio instalado y en ejecucion correctamente!"
    elif [ "$MODE" == "USER" ]; then
        echo "[+] Autostart configurado. Se iniciara en el proximo login o ejecutando: $AGENT_PATH"
    else
        echo "[-] Error al iniciar el servicio. Revisa los logs con: journalctl -u $SERVICE_NAME.service -n 20"
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
