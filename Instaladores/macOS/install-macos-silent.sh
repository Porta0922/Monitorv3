#!/bin/bash
# ActivityMonitor Enterprise v3 - macOS SILENT Installer (USB / AnyDesk)
# Installation completed without user interaction.
# NOTE: TCC permissions (Accessibility & Screen Recording) must be granted manually after.

set -e

if [ "$EUID" -ne 0 ]; then
    echo "[ERR] Este script debe ejecutarse con sudo:"
    echo "    sudo ./install-macos-silent.sh"
    exit 1
fi

AGENT_NAME="activity-monitor-agent"
INSTALL_DIR="/Library/Application Support/ActivityMonitor"
BIN_DIR="$INSTALL_DIR/Bin"
LOG_DIR="$INSTALL_DIR/Logs"
DATA_DIR="/var/lib/activity-monitor"
AGENT_BIN="$BIN_DIR/activity-monitor-agent"
ENV_FILE="$INSTALL_DIR/.env"

DAEMON_PLIST="/Library/LaunchDaemons/com.activitymonitor.daemon.plist"
AGENT_PLIST="/Library/LaunchAgents/com.activitymonitor.agent.plist"

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Resilient USB Paths (Standalone)
if [ -f "$SCRIPT_DIR/activity-monitor-agent" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../agent"
elif [ -f "$SCRIPT_DIR/../agent/Cargo.toml" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../agent"
elif [ -f "$SCRIPT_DIR/agent/Cargo.toml" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/agent"
else
    AGENT_PATH_SRC="$SCRIPT_DIR/../../target/release/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../.."
fi

# Master Credentials (Preconfiguracion para despliegues masivos)
AGENT_AUTH_TOKEN="change-me-in-production"
AGENT_OFFLINE_CACHE_KEY="replace-with-32-byte-cache-key!!"
AGENT_SERVER_URL="http://10.30.0.123:3000"
RABBITMQ_URL="amqp://eclub:eCLUB123@10.30.0.123:5672/%2f"

echo "[*] Iniciando instalacion silenciosa de ActivityMonitor para macOS..."

# 1. Preparar directorios
mkdir -p "$BIN_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 755 "$INSTALL_DIR" "$BIN_DIR"
chmod 1777 "$LOG_DIR" "$DATA_DIR"

# 2. Configurar .env maestro
cat > "$ENV_FILE" << ENVEOF
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=$AGENT_OFFLINE_CACHE_KEY
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
chmod 644 "$ENV_FILE"

# 3. Compilar si no existe pre-compilado
if [ ! -f "$AGENT_PATH_SRC" ]; then
    echo "[*] Binario pre-compilado no encontrado. Compilando silenciosamente..."
    
    # Instalar Rust si no está disponible
    if ! command -v cargo &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1
        source "$HOME/.cargo/env"
    fi
    
    if [ -f "$SRC_PATH/Cargo.toml" ]; then
        pushd "$SRC_PATH" > /dev/null
        cargo build --release >/dev/null 2>&1
        popd > /dev/null
        AGENT_PATH_SRC="$SRC_PATH/target/release/activity-monitor-agent"
        cp "$AGENT_PATH_SRC" "$SCRIPT_DIR/activity-monitor-agent" 2>/dev/null || true
    else
        echo "[-] ERROR: Codigo fuente no encontrado en $SRC_PATH/Cargo.toml"
        echo "    Asegurese de que la carpeta 'agent' este en el USB."
        exit 1
    fi
fi

# 4. Desplegar binario
cp -f "$AGENT_PATH_SRC" "$AGENT_BIN"
chmod 755 "$AGENT_BIN"

# 5. Registrar LaunchDaemon (Sistema)
cat > "$DAEMON_PLIST" << EOL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.activitymonitor.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>$AGENT_BIN</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>$DATA_DIR</string>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/daemon_error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>AGENT_MODE</key>
        <string>SERVICE</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOL
chmod 644 "$DAEMON_PLIST"

# 6. Registrar LaunchAgent (Sesion de Usuario)
cat > "$AGENT_PLIST" << EOL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.activitymonitor.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$AGENT_BIN</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>WorkingDirectory</key>
    <string>$DATA_DIR</string>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/agent.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/agent_error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>AGENT_MODE</key>
        <string>USER</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOL
chmod 644 "$AGENT_PLIST"

# 7. Cargar servicios
launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
launchctl load -w "$DAEMON_PLIST"

LOGGED_USER=$(logname 2>/dev/null || echo $SUDO_USER)
if [ -n "$LOGGED_USER" ] && [ "$LOGGED_USER" != "root" ]; then
    sudo -u "$LOGGED_USER" launchctl unload "$AGENT_PLIST" 2>/dev/null || true
    sudo -u "$LOGGED_USER" launchctl load -w "$AGENT_PLIST"
fi

echo "[+] INSTALACION SILENCIOSA DE macOS COMPLETADA."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⚠️  ACCION MANUAL OBLIGATORIA: Permisos TCC de Apple"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "El usuario debe otorgar MANUALMENTE estos 2 permisos:"
echo ""
echo "  1. Configuracion del Sistema > Privacidad > ACCESIBILIDAD"
echo "     → Agregar: $AGENT_BIN"
echo ""
echo "  2. Configuracion del Sistema > Privacidad > GRABACION DE PANTALLA"
echo "     → Agregar: $AGENT_BIN"
echo ""
echo "  3. REINICIAR la sesion de usuario tras dar los permisos."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
