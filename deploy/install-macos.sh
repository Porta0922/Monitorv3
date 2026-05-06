#!/bin/bash
# ActivityMonitor Enterprise Agent - macOS Installation Script

echo "========================================================="
echo "  Instalador de ActivityMonitor Enterprise Agent (macOS) "
echo "========================================================="
echo ""

# Request root privileges upfront
if [ "$EUID" -ne 0 ]; then
    echo "[-] Por favor, ejecuta este script con sudo:"
    echo "    sudo ./install_macos.sh"
    exit 1
fi

AGENT_NAME="ActivityMonitorAgent"
INSTALL_DIR="/Library/Application Support/ActivityMonitor"
BIN_DIR="$INSTALL_DIR/Bin"
LOG_DIR="$INSTALL_DIR/Logs"
AGENT_BIN="$BIN_DIR/activity-monitor-agent"
ENV_FILE="$INSTALL_DIR/.env"
PLIST_FILE="/Library/LaunchDaemons/com.activitymonitor.agent.plist"

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

echo "[1/7] Verificando dependencias (Rust/Cargo)..."
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

echo "[2/7] Compilando el binario del agente..."
if [ ! -f "$AGENT_PATH" ]; then
    if [ ! -f "$SRC_PATH/Cargo.toml" ]; then
        echo "[-] Codigo fuente no encontrado en $SRC_PATH"
        exit 1
    fi
    echo "[*] Compilando codigo fuente con 'cargo build --release'..."
    cd "$SRC_PATH"
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "[-] Error en la compilacion."
        exit 1
    fi
    AGENT_PATH="$SRC_PATH/target/release/activity-monitor-agent"
    cd "$SCRIPT_DIR"
    
    # Copy compiled binary back to script directory if run from portable USB
    if [ "$SCRIPT_DIR/activity-monitor-agent" != "$AGENT_PATH" ]; then
        cp "$AGENT_PATH" "$SCRIPT_DIR/activity-monitor-agent" 2>/dev/null
    fi
else
    echo "[+] Usando binario pre-compilado: $AGENT_PATH"
fi

echo "[3/7] Preparando directorios..."
mkdir -p "$BIN_DIR"
mkdir -p "$LOG_DIR"
chmod 755 "$INSTALL_DIR"
chmod 755 "$BIN_DIR"
chmod 777 "$LOG_DIR"

echo "[4/7] Configurando archivo .env..."
if [ ! -f "$ENV_FILE" ]; then
    echo "Configurando entorno inicial:"
    read -p "Token de autenticacion (default: change-me-in-production): " input_token
    read -p "URL del Servidor Osquery (default: http://10.30.0.123:3000): " input_server
    read -p "URL de RabbitMQ (default: amqp://eclub:eCLUB123@10.30.0.123:5672/%2f): " input_rabbitmq
    
    cat > "$ENV_FILE" << EOL
AGENT_AUTH_TOKEN=${input_token:-change-me-in-production}
AGENT_SERVER_URL=${input_server:-http://10.30.0.123:3000}
RABBITMQ_URL=${input_rabbitmq:-amqp://eclub:eCLUB123@10.30.0.123:5672/%2f}
EOL
    echo "[+] Archivo .env creado."
else
    echo "[+] Archivo .env existente detectado. Conservando."
fi

chmod 600 "$ENV_FILE"

echo "[5/7] Copiando binario..."
cp "$AGENT_PATH" "$AGENT_BIN"
chmod +x "$AGENT_BIN"

echo "[6/7] Creando LaunchDaemon (Servicio en segundo plano)..."
cat > "$PLIST_FILE" << EOL
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
    <key>StandardOutPath</key>
    <string>$LOG_DIR/agent.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/agent_error.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOL
chmod 644 "$PLIST_FILE"

echo "[7/7] Iniciando el servicio..."
launchctl unload "$PLIST_FILE" 2>/dev/null
launchctl load -w "$PLIST_FILE"

echo ""
echo "========================================================="
echo "  Instalacion completada con exito."
echo "========================================================="
echo "Nota Importante para macOS: "
echo "Para capturar ventanas activas, teclas y raton, debes conceder"
echo "permisos de 'Accesibilidad' y 'Grabacion de pantalla' a:"
echo "1. Tu aplicacion de Terminal (si pruebas manualmente)"
echo "2. El binario en $AGENT_BIN"
echo "Ve a Ajustes del Sistema -> Privacidad y Seguridad -> Accesibilidad"
echo "========================================================="
