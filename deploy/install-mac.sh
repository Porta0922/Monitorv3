#!/bin/bash
# ActivityMonitor Enterprise v3 - macOS Installer
# Creates a LaunchAgent for the current user

set -e

# Configuration
AGENT_NAME="activity-monitor-agent"
SERVICE_NAME="com.activitymonitor.agent"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_BIN="$SCRIPT_DIR/../target/release/$AGENT_NAME"

# macOS User-level directories (required for UI/Accessibility capture)
BASE_DIR="$HOME/.activitymonitor"
AGENT_PATH="$BASE_DIR/bin/$AGENT_NAME"
CONFIG_DIR="$BASE_DIR/config"
LOG_DIR="$BASE_DIR/logs"
DATA_DIR="$BASE_DIR/data"
ENV_FILE="$CONFIG_DIR/.env"
NICKNAME_FILE="$DATA_DIR/device_nickname.txt"
PLIST_PATH="$HOME/Library/LaunchAgents/$SERVICE_NAME.plist"

# Default Variables
AGENT_AUTH_TOKEN="change-me-in-production"
AGENT_SERVER_URL="http://10.30.0.123:3000"
RABBITMQ_URL="amqp://eclub:eCLUB123@10.30.0.123:5672/%2f"
AGENT_OFFLINE_CACHE_KEY="replace-with-32-byte-cache-key!!"

# Check that script is NOT run as root (we want to install as user for UI session)
if [[ $EUID -eq 0 ]]; then
   echo "[-] Este script NO debe ejecutarse con sudo. Ejecutalo como tu usuario normal para poder monitorear la sesion grafica."
   exit 1
fi

# Load existing environment variables if present
if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
fi

show_menu() {
    clear
    echo "========================================"
    echo "ActivityMonitor Enterprise v3.1.0-HYBRID (macOS)"
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
    echo "3. Actualizar binario (Mantiene configuracion)"
    echo "4. Desinstalar"
    echo "5. Salir"
    echo ""
    read -p "Opcion: " MENU_OPTION

    case $MENU_OPTION in
        1) install_agent ;;
        2) modify_creds ;;
        3) update_only ;;
        4) uninstall_agent ;;
        5) exit 0 ;;
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

    if launchctl list | grep -q "$SERVICE_NAME"; then
        echo "[*] Reiniciando LaunchAgent para aplicar cambios..."
        launchctl unload -w "$PLIST_PATH" 2>/dev/null || true
        launchctl load -w "$PLIST_PATH"
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
    if launchctl list | grep -q "$SERVICE_NAME"; then
        echo "[*] Deteniendo LaunchAgent..."
        launchctl unload -w "$PLIST_PATH" 2>/dev/null || true
        echo "[*] Removiendo archivo plist..."
        rm -f "$PLIST_PATH"
    else
        echo "[!] El servicio no esta instalado o registrado en launchd."
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
    echo "[1/5] Preparando directorios..."
    mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
    mkdir -p "$BASE_DIR/bin"
    mkdir -p "$HOME/Library/LaunchAgents"
    echo "    > Directorios listos [OK]"

    echo "[2/5] Configurando archivo .env..."
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

    echo "[3/5] Verificando/Compilando el agente..."
    CARGO_FOUND=0
    if command -v cargo &> /dev/null; then
        CARGO_FOUND=1
    elif [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
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
        echo "[-] Instale Rust toolchain primero usando rustup o copie un ejecutable pre-compilado."
        exit 1
    else
        echo "[+] Usando binario en $TARGET_BIN"
    fi

    echo "[4/5] Desplegando binario en la ruta local..."
    launchctl unload -w "$PLIST_PATH" 2>/dev/null || true
    pkill -f "$AGENT_NAME" || true
    cp -f "$TARGET_BIN" "$AGENT_PATH"
    chmod 755 "$AGENT_PATH"
    echo "    > Binario copiado a $AGENT_PATH [OK]"

    echo "[5/5] Configurando e iniciando LaunchAgent..."
    cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$SERVICE_NAME</string>
    <key>ProgramArguments</key>
    <array>
        <string>$AGENT_PATH</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>AGENT_AUTH_TOKEN</key>
        <string>$AGENT_AUTH_TOKEN</string>
        <key>AGENT_OFFLINE_CACHE_KEY</key>
        <string>$AGENT_OFFLINE_CACHE_KEY</string>
        <key>AGENT_SERVER_URL</key>
        <string>$AGENT_SERVER_URL</string>
        <key>RABBITMQ_URL</key>
        <string>$RABBITMQ_URL</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/output.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/error.log</string>
    <key>WorkingDirectory</key>
    <string>$DATA_DIR</string>
</dict>
</plist>
EOF

    echo ""
    echo "[*] Iniciando servicio..."
    launchctl load -w "$PLIST_PATH"

    if launchctl list | grep -q "$SERVICE_NAME"; then
        echo "[+] Servicio instalado y en ejecucion correctamente!"
        echo "=========================================================="
        echo " NOTA IMPORTANTE PARA macOS:                             "
        echo " Debes otorgar permisos de 'Accesibilidad' al agente.     "
        echo " Ve a Preferencias del Sistema > Seguridad y Privacidad   "
        echo " > Privacidad > Accesibilidad, y añade el archivo:        "
        echo " $AGENT_PATH "
        echo "=========================================================="
    else
        echo "[-] Error al iniciar el servicio con launchd."
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
    echo "- Logs:       $LOG_DIR"
    echo ""
    echo "========================================"
    
    echo ""
    read -n 1 -s -r -p "Presione cualquier tecla para volver al menu..."
    show_menu
}

# Iniciar Menu
show_menu
