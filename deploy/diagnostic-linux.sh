#!/bin/bash
# ActivityMonitor Linux Diagnostic Tool

echo "=========================================="
echo "ActivityMonitor Agent Diagnostic (Linux)"
echo "=========================================="
date
echo ""

# 1. Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "[!] Warning: This script is not running as root. Some checks may fail."
fi

# 2. Load environment variables
ENV_FILE="/etc/activity-monitor/.env"
if [ -f "$ENV_FILE" ]; then
    echo "[+] Loading configuration from $ENV_FILE"
    source "$ENV_FILE"
else
    echo "[-] Error: Configuration file $ENV_FILE not found."
fi

# 3. Check service status
echo ""
echo "=== Service Status ==="
if systemctl is-active --quiet activity-monitor-agent; then
    echo "✅ activity-monitor-agent is running."
else
    echo "❌ activity-monitor-agent is NOT running."
    systemctl status activity-monitor-agent --no-pager
fi

# 4. Check connectivity
echo ""
echo "=== Connectivity Check ==="
if [ -n "$AGENT_SERVER_URL" ]; then
    SERVER_HOST=$(echo $AGENT_SERVER_URL | sed -e 's|^[^/]*//||' -e 's|[:/]..*||')
    SERVER_PORT=$(echo $AGENT_SERVER_URL | sed -e 's|^.*:||' -e 's|/.*||')
    [[ "$SERVER_PORT" == "$SERVER_HOST" ]] && SERVER_PORT=80
    
    echo "[*] Testing API ($SERVER_HOST:$SERVER_PORT)..."
    if timeout 2 bash -c "</dev/tcp/$SERVER_HOST/$SERVER_PORT" 2>/dev/null; then
        echo "    ✅ API reachable."
    else
        echo "    ❌ API UNREACHABLE."
    fi
else
    echo "[-] AGENT_SERVER_URL not set."
fi

if [ -n "$RABBITMQ_URL" ]; then
    RABBIT_HOST=$(echo $RABBITMQ_URL | sed -e 's|^.*@||' -e 's|[:/]..*||')
    RABBIT_PORT=$(echo $RABBITMQ_URL | sed -e 's|^.*:||' -e 's|/.*||')
    
    echo "[*] Testing RabbitMQ ($RABBIT_HOST:$RABBIT_PORT)..."
    if timeout 2 bash -c "</dev/tcp/$RABBIT_HOST/$RABBIT_PORT" 2>/dev/null; then
        echo "    ✅ RabbitMQ reachable."
    else
        echo "    ❌ RabbitMQ UNREACHABLE."
    fi
else
    echo "[-] RABBITMQ_URL not set."
fi

# 5. Check firewall
echo ""
echo "=== Firewall Status (UFW) ==="
if command -v ufw > /dev/null; then
    ufw status
else
    echo "[*] ufw command not found."
fi

# 6. Recent Logs
echo ""
echo "=== Recent Agent Logs ==="
journalctl -u activity-monitor-agent -n 20 --no-pager

echo ""
echo "=========================================="
echo "Diagnostic Complete."
