#!/bin/bash
# ActivityMonitor Enterprise v3 - Linux Installer (USB Portable)
# Creates systemd service unit for agent (Standalone Build)

set -e

# Configuration
AGENT_NAME="activity-monitor-agent"
SERVICE_NAME="activity-monitor-agent"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Resilient USB Paths
if [ -f "$SCRIPT_DIR/$AGENT_NAME" ]; then
    TARGET_BIN="$SCRIPT_DIR/$AGENT_NAME"
    SRC_PATH="$SCRIPT_DIR/../agent"
elif [ -f "$SCRIPT_DIR/../agent/Cargo.toml" ]; then
    TARGET_BIN="$SCRIPT_DIR/$AGENT_NAME"
    SRC_PATH="$SCRIPT_DIR/../agent"
elif [ -f "$SCRIPT_DIR/agent/Cargo.toml" ]; then
    TARGET_BIN="$SCRIPT_DIR/$AGENT_NAME"
    SRC_PATH="$SCRIPT_DIR/agent"
else
    TARGET_BIN="$SCRIPT_DIR/../../target/release/$AGENT_NAME"
    SRC_PATH="$SCRIPT_DIR/../.."
fi

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
FIREWALL_AUTO_FIX=1
CONNECTED_TO_SERVER=0

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
   echo "[-] Este script debe ejecutarse como root. (Use: sudo ./install-linux.sh)"
   exit 1
fi

check_connectivity() {
    echo ""
    echo "=== Verificando Conectividad ==="
    SERVER_HOST=$(echo $AGENT_SERVER_URL | sed -e 's|^[^/]*//||' -e 's|[:/]..*||')
    SERVER_PORT=$(echo $AGENT_SERVER_URL | sed -e 's|^.*:||' -e 's|/.*||')
    [[ "$SERVER_PORT" == "$SERVER_HOST" ]] && SERVER_PORT=80
    RABBIT_HOST=$(echo $RABBITMQ_URL | sed -e 's|^.*@||' -e 's|[:/]..*||')
    RABBIT_PORT=$(echo $RABBITMQ_URL | sed -e 's|^.*:||' -e 's|/.*||')

    echo "[*] Probando conexion a la API ($SERVER_HOST:$SERVER_PORT)..."
    if timeout 2 bash -c "</dev/tcp/$SERVER_HOST/$SERVER_PORT" 2>/dev/null; then
        echo "    [OK] API alcanzable."
        CONNECTED_TO_SERVER=1
    else
        echo "    [ERR] ERROR: No se puede conectar a la API en $SERVER_HOST:$SERVER_PORT"
    fi

    echo "[*] Probando conexion a RabbitMQ ($RABBIT_HOST:$RABBIT_PORT)..."
    if timeout 2 bash -c "</dev/tcp/$RABBIT_HOST/$RABBIT_PORT" 2>/dev/null; then
        echo "    [OK] RabbitMQ alcanzable."
    else
        echo "    [ERR] ERROR: No se puede conectar a RabbitMQ en $RABBIT_HOST:$RABBIT_PORT"
    fi

    if [ $CONNECTED_TO_SERVER -eq 0 ]; then
        echo ""
        read -p "[?] Desea continuar con la instalacion de todos modos? (y/n): " CONTINUE_INSTALL
        if [[ ! $CONTINUE_INSTALL =~ ^[Yy]$ ]]; then
            echo "[-] Instalacion cancelada."
            exit 1
        fi
    fi
}

configure_firewall() {
    if command -v ufw > /dev/null; then
        if ufw status | grep -q "Status: active"; then
            echo "[*] Detectado firewall UFW activo. Configurando reglas..."
            SERVER_PORT=$(echo $AGENT_SERVER_URL | sed -e 's|^.*:||' -e 's|/.*||')
            [[ "$SERVER_PORT" == "$SERVER_HOST" ]] && SERVER_PORT=80
            RABBIT_PORT=$(echo $RABBITMQ_URL | sed -e 's|^.*:||' -e 's|/.*||')
            ufw allow out to any port $SERVER_PORT proto tcp comment 'ActivityMonitor API'
            ufw allow out to any port $RABBIT_PORT proto tcp comment 'ActivityMonitor RabbitMQ'
            echo "    [OK] Reglas de firewall aplicadas."
        fi
    fi
}

if [ -f "$ENV_FILE" ]; then source "$ENV_FILE"; fi

show_menu() {
    clear
    echo "========================================================="
    echo "ActivityMonitor Enterprise v3.3.3 (Linux Installer - USB)"
    echo "========================================================="
    echo "Rutas:"
    echo "- Ejecutable: $AGENT_PATH"
    echo "- Config:     $CONFIG_DIR"
    echo "========================================================="
    echo ""
    echo "1. Instalacion COMPLETA"
    echo "2. Instalar solo SERVICIO"
    echo "3. Instalar solo AUTOSTART USUARIO"
    echo "4. Modificar credenciales (.env)"
    echo "5. Desinstalar"
    echo "6. Salir"
    echo ""
    read -p "Opcion: " MENU_OPTION
    case $MENU_OPTION in
        1) MODE="FULL"; execute_steps ;;
        2) MODE="SERVICE"; execute_steps ;;
        3) MODE="USER"; execute_steps ;;
        4) modify_creds ;;
        5) uninstall_agent ;;
        6) exit 0 ;;
        *) show_menu ;;
    esac
}

modify_creds() {
    read -p "Token (default: $AGENT_AUTH_TOKEN): " INPUT_AUTH_TOKEN
    [ -n "$INPUT_AUTH_TOKEN" ] && AGENT_AUTH_TOKEN=$INPUT_AUTH_TOKEN
    read -p "Server URL (default: $AGENT_SERVER_URL): " INPUT_SERVER_URL
    [ -n "$INPUT_SERVER_URL" ] && AGENT_SERVER_URL=$INPUT_SERVER_URL
    mkdir -p "$CONFIG_DIR"
    cat > "$ENV_FILE" << ENVEOF
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
    chmod 644 "$ENV_FILE"
    systemctl restart $SERVICE_NAME.service 2>/dev/null || true
    read -p "Presione Enter para volver..."
    show_menu
}

uninstall_agent() {
    systemctl stop $SERVICE_NAME.service 2>/dev/null || true
    systemctl disable $SERVICE_NAME.service 2>/dev/null || true
    rm -f /etc/systemd/system/$SERVICE_NAME.service
    systemctl daemon-reload
    pkill -f "$AGENT_NAME" || true
    echo "[+] Desinstalado."
    read -p "Presione Enter para volver..."
    show_menu
}

execute_steps() {
    echo ""
    read -p "Token (default: $AGENT_AUTH_TOKEN): " INPUT_AUTH_TOKEN
    [ -n "$INPUT_AUTH_TOKEN" ] && AGENT_AUTH_TOKEN=$INPUT_AUTH_TOKEN
    read -p "Server (default: $AGENT_SERVER_URL): " INPUT_SERVER_URL
    [ -n "$INPUT_SERVER_URL" ] && AGENT_SERVER_URL=$INPUT_SERVER_URL
    
    check_connectivity

    echo "[1/6] Preparando directorios..."
    mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR" "/opt/activity-monitor/bin"
    
    echo "[2/6] Configurando .env..."
    cat > "$ENV_FILE" << ENVEOF
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
    chmod 644 "$ENV_FILE"

    echo "[3/6] Compilando agente (Standalone - Sin requerir carpeta server)..."
    if [ ! -f "$TARGET_BIN" ]; then
        if ! command -v cargo &> /dev/null; then
            echo "[*] Instalando Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi

        echo "[*] Instalando dependencias de compilacion..."
        if command -v apt-get &> /dev/null; then
            apt-get update -y && apt-get install -y libssl-dev pkg-config build-essential libxtst-dev libx11-dev libwayland-dev libclang-dev libxcb1-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev libegl-dev libxkbcommon-dev libgbm-dev
        fi

        if [ -f "$SRC_PATH/Cargo.toml" ]; then
            pushd "$SRC_PATH" > /dev/null
            echo "[*] Compilando agente localmente de forma independiente..."
            cargo build --release
            popd > /dev/null
            TARGET_BIN="$SRC_PATH/target/release/$AGENT_NAME"
            
            # Copy compiled binary back to script directory if run from portable USB
            if [ "$SCRIPT_DIR/$AGENT_NAME" != "$TARGET_BIN" ]; then
                cp "$TARGET_BIN" "$SCRIPT_DIR/$AGENT_NAME" 2>/dev/null || true
                TARGET_BIN="$SCRIPT_DIR/$AGENT_NAME"
            fi
        else
            echo "[ERR] No se encontro la carpeta con codigo fuente ($SRC_PATH/Cargo.toml)"
            exit 1
        fi
    else
        echo "[+] Usando binario pre-compilado: $TARGET_BIN"
    fi

    if [ ! -f "$TARGET_BIN" ]; then
        echo "[ERR] Fallo la compilacion o no se encontro el binario en $TARGET_BIN."
        exit 1
    fi

    echo "[4/6] Desplegando binario..."
    systemctl stop $SERVICE_NAME.service 2>/dev/null || true
    cp -f "$TARGET_BIN" "$AGENT_PATH"
    chmod 755 "$AGENT_PATH"

    echo "[5/6] Configurando permisos..."
    if command -v apt-get &> /dev/null; then
        echo "[*] Instalando utilidades de X11 (x11-utils) para la captura de ventanas..."
        apt-get update -y && apt-get install -y x11-utils || true
    fi

    if ! id -u activity-monitor &>/dev/null; then
        useradd --system --home $DATA_DIR --shell /usr/sbin/nologin activity-monitor
    fi
    # Determine local desktop user
    CURRENT_USER=$(logname 2>/dev/null || echo $SUDO_USER || echo $USER)
    if [ -n "$CURRENT_USER" ] && [ "$CURRENT_USER" != "root" ]; then
        echo "[*] Agregando usuario '$CURRENT_USER' a los grupos input y netdev..."
        usermod -aG input,netdev "$CURRENT_USER" || true
    fi

    # Validate Wayland environment and force X11 if GDM is used
    if [ -f "/etc/gdm3/custom.conf" ]; then
        echo "[*] Optimizando gestor gdm3 para forzar el uso de X11 (Xorg)..."
        if grep -q "#WaylandEnable=false" /etc/gdm3/custom.conf; then
            sed -i 's/#WaylandEnable=false/WaylandEnable=false/' /etc/gdm3/custom.conf
            echo "    [OK] Desactivado Wayland en /etc/gdm3/custom.conf."
        elif grep -q "WaylandEnable=true" /etc/gdm3/custom.conf; then
            sed -i 's/WaylandEnable=true/WaylandEnable=false/' /etc/gdm3/custom.conf
            echo "    [OK] Forzado WaylandEnable=false en /etc/gdm3/custom.conf."
        elif ! grep -q "WaylandEnable=" /etc/gdm3/custom.conf; then
            sed -i '/\[daemon\]/a WaylandEnable=false' /etc/gdm3/custom.conf
            echo "    [OK] Agregado WaylandEnable=false en /etc/gdm3/custom.conf."
        fi
    fi

    if [ "$XDG_SESSION_TYPE" = "wayland" ] || [ -n "$WAYLAND_DISPLAY" ]; then
        echo ""
        echo "⚠️  ADVERTENCIA: Se detecto una sesion grafica WAYLAND activa en este momento."
        echo "   Hemos configurado el gestor grafico para desactivar Wayland de forma permanente."
        echo "   Para que el cambio surta efecto y empiece a reportar actividad, reinicie el equipo."
        echo ""
    fi

    # Check input group device permissions
    if [ -d "/dev/input" ]; then
        echo "[*] Verificando permisos de dispositivos de entrada en /dev/input..."
        if [ ! -f "/etc/udev/rules.d/99-input.rules" ]; then
            echo 'KERNEL=="event*", NAME="input/%k", MODE="0660", GROUP="input"' > /etc/udev/rules.d/99-input.rules
            udevadm control --reload-rules && udevadm trigger || true
            echo "    [OK] Regla udev para /dev/input configurada."
        fi
    fi

    chown -R root:root "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    chmod 755 "$CONFIG_DIR"
    chmod 1777 "$LOG_DIR" "$DATA_DIR"

    echo "[6/6] Iniciando servicio..."
    configure_firewall
    if [ "$MODE" != "USER" ]; then
        cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=ActivityMonitor Agent
After=network-online.target
[Service]
Type=simple
User=root
WorkingDirectory=$DATA_DIR
EnvironmentFile=-$ENV_FILE
Environment=AGENT_MODE=SERVICE
ExecStart=$AGENT_PATH
Restart=on-failure
[Install]
WantedBy=multi-user.target
EOF
        systemctl daemon-reload
        systemctl enable $SERVICE_NAME.service
        systemctl start $SERVICE_NAME.service
    fi

    if [ "$MODE" != "SERVICE" ]; then
        USER_HOME=$(eval echo ~$SUDO_USER)
        if [ -d "$USER_HOME" ]; then
            AUTOSTART_DIR="$USER_HOME/.config/autostart"
            mkdir -p "$AUTOSTART_DIR"

            # Create user launcher wrapper script
            LAUNCHER_PATH="/opt/activity-monitor/bin/activity-monitor-agent-user-launcher.sh"
            echo "[*] Creando script launcher para la sesion de usuario..."
            cat > "$LAUNCHER_PATH" << 'LAUNCHEREOF'
#!/bin/bash
# Auto-detect display session for activity monitor user agent
export DISPLAY="${DISPLAY:-:0}"
export XAUTHORITY="${XAUTHORITY:-$HOME/.Xauthority}"
# Also try to get DISPLAY from active X11 sessions
if [ -z "$DISPLAY" ] || ! xprop -root &>/dev/null 2>&1; then
    XSESSION=$(w -h $(whoami) 2>/dev/null | awk '{print $3}' | grep -E '^:[0-9]' | head -1)
    export DISPLAY="${XSESSION:-:0}"
fi
exec /opt/activity-monitor/bin/activity-monitor-agent
LAUNCHEREOF
            chmod 755 "$LAUNCHER_PATH"

            cat > "$AUTOSTART_DIR/activity-monitor.desktop" << EOF
[Desktop Entry]
Type=Application
Name=ActivityMonitor Agent
Comment=ActivityMonitor Telemetry User Agent
Exec=env AGENT_MODE=USER /opt/activity-monitor/bin/activity-monitor-agent-user-launcher.sh
Terminal=false
Categories=Utility;
X-GNOME-Autostart-enabled=true
EOF
            chown -R $SUDO_USER:$SUDO_USER "$USER_HOME/.config"

            # Launch the agent immediately as the logged-in user (without waiting for reboot/logout)
            if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
                echo "[*] Iniciando el agente de usuario inmediatamente para '$SUDO_USER'..."
                pkill -u "$SUDO_USER" -f "activity-monitor-agent" || true
                su - "$SUDO_USER" -c "env AGENT_MODE=USER DISPLAY=:0 /opt/activity-monitor/bin/activity-monitor-agent-user-launcher.sh >/dev/null 2>&1 &" &
            fi
        else
            echo "⚠️  No se pudo encontrar la carpeta personal de $SUDO_USER para configurar Autostart."
        fi
    fi

    echo "========================================"
    echo "INSTALACION COMPLETADA EXITOSAMENTE"
    echo "IMPORTANTE: Reinicie la sesion de usuario para activar los permisos de hardware."
    echo "========================================"
    read -p "Presione Enter para finalizar..."
    exit 0
}

show_menu
