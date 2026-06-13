#!/bin/bash
# ActivityMonitor Enterprise v3 - macOS Installer (USB Portable)
# Supports LaunchDaemon (System Service) and LaunchAgent (User Session Telemetry)
# Standalone Build: Does NOT require the server/ folder.

echo "========================================================="
echo "  ActivityMonitor Enterprise v3.3.3 - Instalador macOS  "
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

# Resilient USB Paths
if [ -f "$SCRIPT_DIR/activity-monitor-agent" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../../agent"
elif [ -f "$SCRIPT_DIR/../../agent/Cargo.toml" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../../agent"
elif [ -f "$SCRIPT_DIR/../agent/Cargo.toml" ]; then
    AGENT_PATH_SRC="$SCRIPT_DIR/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../agent"
else
    AGENT_PATH_SRC="$SCRIPT_DIR/../../target/release/activity-monitor-agent"
    SRC_PATH="$SCRIPT_DIR/../.."
fi

echo "[1/8] Verificando dependencias (Rust/Cargo)..."
if ! command -v cargo &> /dev/null; then
    echo "[-] Rust no esta instalado."
    echo "[*] Instalando Rust..."
    # Check Xcode CLI tools first (required by Rust on macOS)
    if ! xcode-select -p &>/dev/null; then
        echo "[*] Instalando Xcode Command Line Tools (requerido para compilar)..."
        xcode-select --install
        echo "[*] Por favor, acepte la instalacion de las herramientas de Xcode en el dialogo del sistema"
        echo "[*] y luego presione Enter para continuar cuando la instalacion haya terminado..."
        read -p ""
    fi
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    if ! command -v cargo &> /dev/null; then
        echo "[-] Error instalando Rust. Por favor, instalalo manualmente: https://rustup.rs/"
        exit 1
    fi
else
    echo "[+] Rust/Cargo detectado."
fi

echo "[2/8] Compilando el binario del agente (Standalone)..."
if [ ! -f "$AGENT_PATH_SRC" ]; then
    if [ ! -f "$SRC_PATH/Cargo.toml" ]; then
        echo "[-] Codigo fuente no encontrado en $SRC_PATH"
        echo "    Asegurese de que la carpeta 'agent' este presente en el USB."
        exit 1
    fi
    echo "[*] Compilando de forma independiente con 'cargo build --release'..."
    echo "[*] Este proceso puede tomar varios minutos la primera vez..."
    pushd "$SRC_PATH" > /dev/null
    cargo build --release
    popd > /dev/null
    BUILT_BIN="$SRC_PATH/target/release/activity-monitor-agent"
    
    if [ ! -f "$BUILT_BIN" ]; then
        echo "[-] Error: No se genero el binario en $BUILT_BIN"
        exit 1
    fi
    
    # Cache the binary next to the script for future installs
    cp "$BUILT_BIN" "$SCRIPT_DIR/activity-monitor-agent" 2>/dev/null || true
    AGENT_PATH_SRC="$BUILT_BIN"
    echo "[+] Binario compilado exitosamente."
else
    echo "[+] Usando binario pre-compilado: $AGENT_PATH_SRC"
fi

echo "[3/8] Preparando directorios..."
mkdir -p "$BIN_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$DATA_DIR"

chmod 755 "$INSTALL_DIR"
chmod 755 "$BIN_DIR"
chmod 1777 "$LOG_DIR"
chmod 1777 "$DATA_DIR"
echo "[+] Directorios creados con permisos correctos."

echo "[4/8] Configurando archivo .env..."
if [ ! -f "$ENV_FILE" ]; then
    echo "Configurando entorno inicial:"
    read -p "  Token de autenticacion (Enter para usar 'change-me-in-production'): " input_token
    read -p "  URL del Servidor (Enter para usar 'http://10.30.0.123:3000'): " input_server
    read -p "  URL de RabbitMQ (Enter para usar default): " input_rabbitmq
    
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
chmod 644 "$ENV_FILE"

echo "[5/8] Copiando binario..."
cp -f "$AGENT_PATH_SRC" "$AGENT_BIN"
chmod 755 "$AGENT_BIN"
echo "[+] Binario copiado a $AGENT_BIN"

echo "[6/8] Registrando LaunchDaemon (Servicio de Sistema - root)..."
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
echo "[+] LaunchDaemon registrado."

echo "[7/8] Registrando LaunchAgent (Sesion Grafica de Usuario)..."
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
echo "[+] LaunchAgent registrado."

echo "[8/8] Cargando e Iniciando servicios..."
launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
launchctl load -w "$DAEMON_PLIST"

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
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "⚠️  PASO CRITICO OBLIGATORIO: Permisos de Privacidad TCC"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "macOS bloquea por defecto la captura de actividad de teclado,"
echo "raton y titulos de ventanas por motivos de privacidad."
echo "DEBES otorgar los siguientes permisos manualmente:"
echo ""
echo "1. ACCESIBILIDAD (Permite rastrear inactividad y eventos globales)"
echo "   → Configuracion del Sistema > Privacidad y Seguridad > Accesibilidad"
echo "   → Haz clic en '+' y agrega:"
echo "     $AGENT_BIN"
echo ""
echo "2. GRABACION DE PANTALLA (Permite leer titulos de ventanas activas)"
echo "   ℹ️  El agente NO toma capturas. Solo lee el nombre de la ventana activa."
echo "   → Configuracion del Sistema > Privacidad y Seguridad > Grabacion de Pantalla"
echo "   → Haz clic en '+' y agrega:"
echo "     $AGENT_BIN"
echo ""
echo "3. TRAS DAR LOS PERMISOS: Cierra la sesion y vuelve a iniciarla"
echo "   (o reinicia la Mac) para que los permisos TCC surtan efecto."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

read -p "¿Desea abrir los paneles de Privacidad y Seguridad ahora? (y/n) [y]: " OPEN_PANELS
if [[ -z "$OPEN_PANELS" || "$OPEN_PANELS" =~ ^[Yy]$ ]]; then
    echo "[*] Abriendo panel de Accesibilidad..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    sleep 1
    echo "[*] Abriendo panel de Grabacion de Pantalla..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
fi

echo ""
echo "========================================================="
