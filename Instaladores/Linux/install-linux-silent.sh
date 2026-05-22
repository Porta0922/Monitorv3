#!/bin/bash
# ActivityMonitor Enterprise v3 - Linux SILENT Installer (USB / SSH / AnyDesk)
# Installation completed in 2 seconds without user interaction.

set -e

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
   echo "[ERR] Este script debe ejecutarse como root (Use: sudo ./install-linux-silent.sh)"
   exit 1
fi

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

# Master Credentials (Preconfiguracion para despliegues masivos)
AGENT_AUTH_TOKEN="change-me-in-production"
AGENT_OFFLINE_CACHE_KEY="replace-with-32-byte-cache-key!!"
AGENT_SERVER_URL="http://10.30.0.123:3000"
RABBITMQ_URL="amqp://eclub:eCLUB123@10.30.0.123:5672/%2f"

echo "[*] Iniciando instalacion silenciosa de ActivityMonitor..."

# 1. Preparar directorios
mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR" "/opt/activity-monitor/bin"

# 2. Configurar .env maestro
cat > "$ENV_FILE" << ENVEOF
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
chmod 644 "$ENV_FILE"

# 3. Compilar si no existe pre-compilado
if [ ! -f "$TARGET_BIN" ]; then
    echo "[*] Binario pre-compilado no encontrado. Iniciando compilacion silenciosa..."
    if ! command -v cargo &> /dev/null; then
        echo "[*] Instalando Rust de forma silenciosa..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1
        source "$HOME/.cargo/env"
    fi
    
    if command -v apt-get &> /dev/null; then
        apt-get update -y >/dev/null 2>&1
        apt-get install -y libssl-dev pkg-config build-essential libxtst-dev libx11-dev libwayland-dev libclang-dev libxcb1-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev >/dev/null 2>&1
    fi

    if [ -f "$SRC_PATH/Cargo.toml" ]; then
        pushd "$SRC_PATH" > /dev/null
        cargo build --release >/dev/null 2>&1
        popd > /dev/null
        TARGET_BIN="$SRC_PATH/target/release/$AGENT_NAME"
    else
        echo "[-] ERROR: Codigo fuente no encontrado en $SRC_PATH/Cargo.toml"
        exit 1
    fi
fi

# 4. Desplegar binario
systemctl stop $SERVICE_NAME.service 2>/dev/null || true
cp -f "$TARGET_BIN" "$AGENT_PATH"
chmod 755 "$AGENT_PATH"

# 5. Configurar permisos especiales y grupos de hardware de entrada
if command -v apt-get &> /dev/null; then
    apt-get update -y && apt-get install -y x11-utils >/dev/null 2>&1 || true
fi

if ! id -u activity-monitor &>/dev/null; then
    useradd --system --home $DATA_DIR --shell /usr/sbin/nologin activity-monitor
fi

CURRENT_USER=$(logname 2>/dev/null || echo $SUDO_USER || echo $USER)
if [ -n "$CURRENT_USER" ] && [ "$CURRENT_USER" != "root" ]; then
    usermod -aG input,netdev "$CURRENT_USER" || true
fi

# Regla udev para permitir rastreo de inactividad al grupo input
if [ -d "/dev/input" ]; then
    if [ ! -f "/etc/udev/rules.d/99-input.rules" ]; then
        echo 'KERNEL=="event*", NAME="input/%k", MODE="0660", GROUP="input"' > /etc/udev/rules.d/99-input.rules
        udevadm control --reload-rules && udevadm trigger || true
    fi
fi

# Desactivar Wayland para forzar X11 globalmente si se detecta gdm3
if [ -f "/etc/gdm3/custom.conf" ]; then
    if grep -q "#WaylandEnable=false" /etc/gdm3/custom.conf; then
        sed -i 's/#WaylandEnable=false/WaylandEnable=false/' /etc/gdm3/custom.conf
    elif grep -q "WaylandEnable=true" /etc/gdm3/custom.conf; then
        sed -i 's/WaylandEnable=true/WaylandEnable=false/' /etc/gdm3/custom.conf
    elif ! grep -q "WaylandEnable=" /etc/gdm3/custom.conf; then
        sed -i '/\[daemon\]/a WaylandEnable=false' /etc/gdm3/custom.conf
    fi
fi

chown -R root:root "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 755 "$CONFIG_DIR"
chmod 1777 "$LOG_DIR" "$DATA_DIR"

# 6. Registrar e Iniciar Servicio de Sistema (systemd)
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
systemctl enable $SERVICE_NAME.service >/dev/null 2>&1
systemctl start $SERVICE_NAME.service >/dev/null 2>&1

# 7. Registrar Autostart de Sesion Gráfica del Usuario
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    USER_HOME=$(eval echo ~$SUDO_USER)
    if [ -d "$USER_HOME" ]; then
        AUTOSTART_DIR="$USER_HOME/.config/autostart"
        mkdir -p "$AUTOSTART_DIR"

        # Create user launcher wrapper script
        LAUNCHER_PATH="/opt/activity-monitor/bin/activity-monitor-agent-user-launcher.sh"
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
        pkill -u "$SUDO_USER" -f "activity-monitor-agent" || true
        su - "$SUDO_USER" -c "env AGENT_MODE=USER DISPLAY=:0 /opt/activity-monitor/bin/activity-monitor-agent-user-launcher.sh >/dev/null 2>&1 &" &
    fi
fi

echo "[+] INSTALACION COMPLETA Y SILENCIOSA DE LINUX FINALIZADA."
echo "[*] IMPORTANTE: Se requiere reiniciar la sesion de usuario para aplicar los permisos de hardware de entrada."
exit 0
