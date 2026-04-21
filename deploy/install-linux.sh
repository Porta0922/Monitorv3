#!/bin/bash
# ActivityMonitor Enterprise v3 - Linux Installer
# Creates systemd service unit for agent

set -e

AGENT_NAME="activity-monitor-agent"
SERVICE_NAME="activity-monitor-agent"
AGENT_PATH="/opt/activity-monitor/bin/$AGENT_NAME"
CONFIG_DIR="/etc/activity-monitor"
LOG_DIR="/var/log/activity-monitor"
DATA_DIR="/var/lib/activity-monitor"
ENV_FILE="$CONFIG_DIR/.env"
NICKNAME_FILE="$DATA_DIR/device_nickname.txt"

echo "========================================"
echo "ActivityMonitor Enterprise v3 Installer"
echo "========================================"
echo

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
   echo "[-] This script must be run as root"
   exit 1
fi

# Ask for device nickname
read -p "Enter device nickname (or press Enter for hostname): " DEVICE_NICKNAME
if [ -z "$DEVICE_NICKNAME" ]; then
    DEVICE_NICKNAME=$(hostname)
    echo "Using hostname: $DEVICE_NICKNAME"
fi

# Ask for optional auth token (used by current agent)
read -p "Enter agent auth token (or press Enter for default dev-agent-token): " AGENT_AUTH_TOKEN
if [ -z "$AGENT_AUTH_TOKEN" ]; then
    AGENT_AUTH_TOKEN="dev-agent-token"
fi

read -p "Enter server URL for remote osquery policy (or press Enter for default http://localhost:3000): " AGENT_SERVER_URL
if [ -z "$AGENT_SERVER_URL" ]; then
    AGENT_SERVER_URL="http://localhost:3000"
fi

read -p "Enter RabbitMQ URL (or press Enter for default amqp://guest:guest@127.0.0.1:5672/%2f): " RABBITMQ_URL
if [ -z "$RABBITMQ_URL" ]; then
    RABBITMQ_URL="amqp://guest:guest@127.0.0.1:5672/%2f"
fi

# Create directories
echo "[*] Creating directories..."
mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
mkdir -p /opt/activity-monitor/bin

# Check if agent binary exists
if [ ! -f "$AGENT_PATH" ]; then
    echo "[-] Agent binary not found at $AGENT_PATH"
    echo "[!] Please build and copy the agent binary first"
    exit 1
fi
echo "[+] Found agent binary: $AGENT_PATH"

# Create .env file if it doesn't exist
if [ ! -f "$ENV_FILE" ]; then
    echo "[*] Creating configuration file..."
    cat > "$ENV_FILE" << ENVEOF
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AGENT_AUTH_TOKEN
AGENT_OFFLINE_CACHE_KEY=replace-with-32-byte-cache-key!!
AGENT_SERVER_URL=$AGENT_SERVER_URL
RABBITMQ_URL=$RABBITMQ_URL
ENVEOF
    chmod 600 "$ENV_FILE"
    echo "[+] Created configuration: $ENV_FILE"
else
    echo "[!] Configuration file already exists"
fi

# Current agent reads nickname from file, not environment variables
echo "$DEVICE_NICKNAME" > "$NICKNAME_FILE"
echo "[+] Saved nickname to: $NICKNAME_FILE"

# Create systemd service unit
echo "[*] Creating systemd service unit..."
cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=ActivityMonitor Enterprise Agent
Documentation=https://github.com/yourrepo/ActivityMonitor-Enterprise-v3
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=activity-monitor
Group=activity-monitor
WorkingDirectory=$DATA_DIR
StandardOutput=journal
StandardError=journal
SyslogIdentifier=$SERVICE_NAME

# Security
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes

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

# Create service user
if ! id -u activity-monitor &>/dev/null; then
    echo "[*] Creating service user..."
    useradd --system --home $DATA_DIR --shell /bin/false activity-monitor
fi

# Set permissions
chown -R activity-monitor:activity-monitor "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 750 "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 755 "$AGENT_PATH"
chmod 640 "$NICKNAME_FILE"

# Reload systemd
echo "[*] Reloading systemd daemon..."
systemctl daemon-reload

# Start service
echo "[*] Starting service..."
systemctl start $SERVICE_NAME.service
systemctl enable $SERVICE_NAME.service

if systemctl is-active --quiet $SERVICE_NAME.service; then
    echo "[+] Service started successfully!"
else
    echo "[-] Failed to start service. Check logs:"
    journalctl -u $SERVICE_NAME.service -n 20
    exit 1
fi

echo
echo "========================================"
echo "Installation Complete"
echo "========================================"
echo "Service Name: $SERVICE_NAME"
echo "Binary Path: $AGENT_PATH"
echo "Config Dir: $CONFIG_DIR"
echo "Log Dir: $LOG_DIR"
echo "Data Dir: $DATA_DIR"
echo
echo "To manage the service:"
echo "  Start:   systemctl start $SERVICE_NAME"
echo "  Stop:    systemctl stop $SERVICE_NAME"
echo "  Restart: systemctl restart $SERVICE_NAME"
echo "  Status:  systemctl status $SERVICE_NAME"
echo "  Logs:    journalctl -u $SERVICE_NAME -f"
echo
