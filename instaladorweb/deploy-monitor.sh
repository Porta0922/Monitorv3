#!/bin/bash

# deploy-monitor.sh - Script de deployment automático para Monitor v3
# Uso: ./deploy-monitor.sh [linux|macos-x86|macos-arm]
# 
# Variables de entorno requeridas:
#   MONITOR_DEPLOY_TOKEN - Token de acceso de GitHub (solo lectura)
#   MONITOR_REPO - Repositorio (default: Porta0922/Monitorv3)

set -e

# Configuración
REPO="${MONITOR_REPO:-Porta0922/Monitorv3}"
TOKEN="${MONITOR_DEPLOY_TOKEN}"
INSTALL_DIR="/opt/monitor"
CONFIG_DIR="/etc/monitor"
SERVICE_NAME="monitor"
LOG_DIR="/var/log/monitor"

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Funciones helper
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Verificar token
if [ -z "$TOKEN" ]; then
    log_error "Token no configurado. Usa: export MONITOR_DEPLOY_TOKEN='tu_token'"
    exit 1
fi

# Detectar plataforma si no se especifica
if [ -z "$1" ]; then
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        PLATFORM="linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        if [[ $(uname -m) == "arm64" ]]; then
            PLATFORM="macos-arm"
        else
            PLATFORM="macos-x86"
        fi
    else
        log_error "Plataforma no soportada"
        exit 1
    fi
else
    PLATFORM=$1
fi

log_info "Iniciando deployment para: $PLATFORM"

# Seleccionar asset según plataforma
case $PLATFORM in
    linux)
        ASSET_NAME="monitor-linux-x86_64"
        ;;
    macos-x86)
        ASSET_NAME="monitor-macos-x86_64"
        ;;
    macos-arm)
        ASSET_NAME="monitor-macos-aarch64"
        ;;
    *)
        log_error "Plataforma desconocida: $PLATFORM"
        exit 1
        ;;
esac

log_info "Descargando información de release..."

# Obtener la última release
RELEASE_JSON=$(curl -s -H "Authorization: token $TOKEN" \
    "https://api.github.com/repos/$REPO/releases/latest")

# Verificar si hubo error en la API
if echo "$RELEASE_JSON" | grep -q "message.*API rate limit"; then
    log_error "Límite de API de GitHub alcanzado. Intenta más tarde."
    exit 1
fi

if echo "$RELEASE_JSON" | grep -q "Not Found"; then
    log_error "No se encontró ninguna release. Crea una con git tag"
    exit 1
fi

# Obtener URL de descarga
DOWNLOAD_URL=$(echo "$RELEASE_JSON" | grep -o "\"browser_download_url\": \"[^\"]*$ASSET_NAME[^\"]*\"" | head -1 | cut -d'"' -f4)

if [ -z "$DOWNLOAD_URL" ]; then
    log_error "No se encontró el asset: $ASSET_NAME"
    log_info "Assets disponibles:"
    echo "$RELEASE_JSON" | grep "browser_download_url" | head -5
    exit 1
fi

log_info "URL de descarga: $DOWNLOAD_URL"

# Crear directorios
log_info "Creando directorios..."
sudo mkdir -p "$INSTALL_DIR"
sudo mkdir -p "$CONFIG_DIR"
sudo mkdir -p "$LOG_DIR"

# Descargar binario
log_info "Descargando binario..."
TEMP_FILE="/tmp/monitor_download"
curl -s -L -H "Authorization: token $TOKEN" "$DOWNLOAD_URL" -o "$TEMP_FILE"

if [ ! -f "$TEMP_FILE" ]; then
    log_error "Error descargando el archivo"
    exit 1
fi

# Instalar binario
log_info "Instalando binario..."
sudo mv "$TEMP_FILE" "$INSTALL_DIR/monitor"
sudo chmod +x "$INSTALL_DIR/monitor"
sudo chown root:root "$INSTALL_DIR/monitor"

log_success "Binario instalado en: $INSTALL_DIR/monitor"

# Crear archivo de configuración si no existe
if [ ! -f "$CONFIG_DIR/monitor.conf" ]; then
    log_info "Creando archivo de configuración..."
    sudo tee "$CONFIG_DIR/monitor.conf" > /dev/null <<EOF
# Monitor v3 Configuration
# Generated: $(date)
# Repository: $REPO

# Puerto para el servidor web (default: 8080)
PORT=8080

# Nivel de logging (debug, info, warn, error)
LOG_LEVEL=info

# Directorio de logs
LOG_DIR=$LOG_DIR

# Intervalo de monitoreo en segundos
CHECK_INTERVAL=60

# Dirección de escucha (0.0.0.0 para todas las interfaces)
BIND_ADDRESS=0.0.0.0
EOF
    sudo chmod 600 "$CONFIG_DIR/monitor.conf"
    log_success "Archivo de configuración creado"
else
    log_warning "El archivo de configuración ya existe, omitiendo..."
fi

# Crear servicio systemd (para Linux)
if [[ "$PLATFORM" == "linux" ]]; then
    log_info "Configurando servicio systemd..."
    sudo tee /etc/systemd/system/monitor.service > /dev/null <<EOF
[Unit]
Description=Monitor v3 Service
After=network.target
Documentation=https://github.com/Porta0922/Monitorv3

[Service]
Type=simple
User=root
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/monitor
EnvironmentFile=$CONFIG_DIR/monitor.conf
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=monitor

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable monitor
    sudo systemctl restart monitor
    
    sleep 2
    
    STATUS=$(sudo systemctl is-active monitor)
    if [ "$STATUS" = "active" ]; then
        log_success "Servicio iniciado correctamente"
    else
        log_warning "Servicio puede no estar activo, verificando..."
        sudo systemctl status monitor
    fi
fi

# Crear launch agent (para macOS)
if [[ "$PLATFORM" == "macos-x86" ]] || [[ "$PLATFORM" == "macos-arm" ]]; then
    log_info "Configurando launch agent macOS..."
    PLIST_PATH="$HOME/Library/LaunchAgents/com.monitor.plist"
    
    mkdir -p "$HOME/Library/LaunchAgents"
    
    cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.monitor.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/monitor</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>$HOME/Library/Logs/monitor.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/Library/Logs/monitor.err</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOF

    chmod 600 "$PLIST_PATH"
    launchctl load "$PLIST_PATH"
    
    sleep 2
    log_success "Servicio iniciado en macOS"
fi

# Mostrar información de debug
echo ""
log_info "=== Información de Deployment ==="
echo "Plataforma: $PLATFORM"
echo "Repositorio: $REPO"
echo "Asset: $ASSET_NAME"
echo "Instalación: $INSTALL_DIR/monitor"
echo "Configuración: $CONFIG_DIR/monitor.conf"
echo "Logs: $LOG_DIR"
echo ""

if [[ "$PLATFORM" == "linux" ]]; then
    log_info "Ver estado del servicio:"
    echo "  sudo systemctl status monitor"
    echo ""
    log_info "Ver logs:"
    echo "  sudo journalctl -u monitor -f"
    echo ""
    log_info "Reiniciar servicio:"
    echo "  sudo systemctl restart monitor"
else
    log_info "Ver logs:"
    echo "  tail -f ~/Library/Logs/monitor.log"
    echo ""
    log_info "Detener servicio:"
    echo "  launchctl unload ~/Library/LaunchAgents/com.monitor.plist"
fi

echo ""
log_success "✨ ¡Deployment completado exitosamente!"
