#!/bin/bash
# ActivityMonitor Enterprise Agent - macOS Installation Script
# Supports Dual LaunchDaemon (System Service) and LaunchAgent (User Session Telemetry)

echo "========================================================="
echo "  Instalador de ActivityMonitor Enterprise Agent (macOS) "
echo "========================================================="
echo ""

# Request root privileges upfront
if [ "$EUID" -ne 0 ]; then
    echo "[-] Por favor, ejecuta este script con sudo:"
    echo "    sudo ./install-macos.sh"
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

# Determine path to binary or source
if [ -f "$SCRIPT_DIR/activity-monitor-agent" ]; then
    AGENT_PATH="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/agent"
elif [ -f "$SCRIPT_DIR/agent/Cargo.toml" ]; then
    AGENT_PATH="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/agent"
else
    AGENT_PATH="$SCRIPT_DIR/../target/release/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/.."
fi

echo "[1/8] Verificando dependencias (Rust/Cargo)..."
if ! command -v cargo &> /dev/null; then
    echo "[-] Rust no esta instalado."
    echo "[*] Instalando Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    if ! command -v cargo &> /dev/null; then
        echo "[-] Error instalando Rust. Por favor, instalalo manualmente: https://rustup.rs/"
        exit 1
    fi
else
    echo "[+] Rust/Cargo detectado."
fi

echo "[2/8] Compilando el binario del agente..."
if [ ! -f "$AGENT_PATH" ]; then
    if [ ! -f "$SRC_PATH/Cargo.toml" ]; then
        echo "[-] Codigo fuente no encontrado en $SRC_PATH"
        exit 1
    fi
    echo "[*] Compilando codigo fuente con 'cargo build --release'..."
    cd "$SRC_PATH"
    # Compile workspace or package
    if [ -f "$SRC_PATH/../Cargo.toml" ]; then
        cd "$SRC_PATH/.."
        cargo build --release -p activity-monitor-agent
        AGENT_PATH="$SRC_PATH/../target/release/activity-monitor-agent"
    else
        cargo build --release
        AGENT_PATH="$SRC_PATH/target/release/activity-monitor-agent"
    fi
    cd "$SCRIPT_DIR"
    
    # Copy compiled binary back to script directory if run from portable USB
    if [ "$SCRIPT_DIR/activity-monitor-agent" != "$AGENT_PATH" ]; then
        cp "$AGENT_PATH" "$SCRIPT_DIR/activity-monitor-agent" 2>/dev/null || true
    fi
else
    echo "[+] Usando binario pre-compilado: $AGENT_PATH"
fi

echo "[3/8] Preparando directorios..."
mkdir -p "$BIN_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$DATA_DIR"

# Set up permissions: make logs and data directories sticky/writeable by any session user
# so that LaunchAgent can create and read the SQLite agent_user_cache.db database file
chmod 755 "$INSTALL_DIR"
chmod 755 "$BIN_DIR"
chmod 1777 "$LOG_DIR"
chmod 1777 "$DATA_DIR"
echo "[+] Directorios creados y configurados con permisos de acceso cruzados."

echo "[4/8] Configurando archivo .env..."
if [ ! -f "$ENV_FILE" ]; then
    echo "Configurando entorno inicial:"
    read -p "Token de autenticacion (default: change-me-in-production): " input_token
    read -p "URL del Servidor (default: http://10.30.0.123:3000): " input_server
    read -p "URL de RabbitMQ (default: amqp://eclub:eCLUB123@10.30.0.123:5672/%2f): " input_rabbitmq
    
    cat > "$ENV_FILE" << EOL
AGENT_AUTH_TOKEN=${input_token:-change-me-in-production}
AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
AGENT_SERVER_URL=${input_server:-http://10.30.0.123:3000}
RABBITMQ_URL=${input_rabbitmq:-amqp://eclub:eCLUB123@10.30.0.123:5672/%2f}
EOL
    echo "[+] Archivo .env creado."
else
    echo "[+] Archivo .env existente detectado. Conservando."
fi

# Must be readable by logged-in users running the LaunchAgent
chmod 644 "$ENV_FILE"

echo "[5/8] Copiando binario..."
cp -f "$AGENT_PATH" "$AGENT_BIN"
chmod 755 "$AGENT_BIN"

echo "[6/8] Registrando LaunchDaemon (Servicio de Sistema - UID 0)..."
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

echo "[7/8] Registrando LaunchAgent (Sesion Gráfica de Usuario - UID > 0)..."
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

echo "[8/8] Cargando e Iniciando servicios..."
# Load LaunchDaemon as root
echo "[*] Iniciando servicio del sistema (Daemon)..."
launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
launchctl load -w "$DAEMON_PLIST"

# Load LaunchAgent for current logged-in desktop user
LOGGED_USER=$(logname 2>/dev/null || echo $SUDO_USER)
if [ -n "$LOGGED_USER" ] && [ "$LOGGED_USER" != "root" ]; then
    USER_UID=$(id -u "$LOGGED_USER")
    echo "[*] Iniciando agente de usuario para $LOGGED_USER (UID $USER_UID)..."
    sudo -u "$LOGGED_USER" launchctl unload "$AGENT_PLIST" 2>/dev/null || true
    sudo -u "$LOGGED_USER" launchctl load -w "$AGENT_PLIST"
fi

echo ""
echo "========================================================="
echo "  Instalacion completada con exito."
echo "========================================================="
echo ""
echo "⚠️  CRÍTICO: IMPORTANTE PARA macOS (Permisos TCC)"
echo "Para capturar eventos de teclado, ratón y foco de ventana,"
echo "debes conceder permisos de sistema a la aplicacion."
echo ""
echo "1. Conceder permisos de ACCESIBILIDAD a:"
echo "   - $AGENT_BIN"
echo "   - Tu aplicación de Terminal (si realizas pruebas manuales)"
echo ""
echo "2. Conceder permisos de GRABACIÓN DE PANTALLA a:"
echo "   - $AGENT_BIN"
echo ""

# Ask to open preferences panels automatically
read -p "¿Desea abrir los paneles de Privacidad y Seguridad ahora? (y/n) [y]: " OPEN_PANELS
if [[ -z "$OPEN_PANELS" || "$OPEN_PANELS" =~ ^[Yy]$ ]]; then
    echo "[*] Abriendo panel de Accesibilidad..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    sleep 1
    echo "[*] Abriendo panel de Grabacion de Pantalla..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
fi

echo ""
echo "Recuerde reiniciar la sesión de usuario si nota que algún evento no se registra."
echo "========================================================="
