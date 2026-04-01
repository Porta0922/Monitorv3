#!/bin/bash
# ActivityMonitor Enterprise v3 - macOS Installer
# Creates launchd plist for agent

AGENT_NAME="activity-monitor-agent"
SERVICE_NAME="com.activitymonitor.agent"
PLIST_PATH="/Library/LaunchDaemons/$SERVICE_NAME.plist"
AGENT_PATH="/usr/local/bin/$AGENT_NAME"
CONFIG_DIR="/etc/activity-monitor"
LOG_DIR="/var/log/activity-monitor"
DATA_DIR="/var/lib/activity-monitor"

echo "========================================"
echo "ActivityMonitor Enterprise v3 Installer"
echo "========================================"
echo

# Check for root privileges
if [[ $EUID -ne 0 ]]; then
   echo "[-] This script must be run as root (use sudo)"
   exit 1
fi

# Create directories
echo "[*] Creating directories..."
mkdir -p "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"

# Check if agent binary exists
if [ ! -f "$AGENT_PATH" ]; then
    echo "[-] Agent binary not found at $AGENT_PATH"
    echo "[!] Please build and copy the agent binary first"
    exit 1
fi
echo "[+] Found agent binary: $AGENT_PATH"

# Create launchd plist
echo "[*] Creating launchd plist..."
cat > "$PLIST_PATH" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.activitymonitor.agent</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/activity-monitor-agent</string>
    </array>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <true/>
    
    <key>StandardOutPath</key>
    <string>/var/log/activity-monitor/output.log</string>
    
    <key>StandardErrorPath</key>
    <string>/var/log/activity-monitor/error.log</string>
    
    <key>WorkingDirectory</key>
    <string>/var/lib/activity-monitor</string>
    
    <key>UserName</key>
    <string>_activitymonitor</string>
    
    <key>GroupName</key>
    <string>_activitymonitor</string>
    
    <key>ProcessType</key>
    <string>Background</string>
    
    <key>Nice</key>
    <integer>5</integer>
    
    <key>ThrottleInterval</key>
    <integer>10</integer>
</dict>
</plist>
EOF

# Create service user
if ! dscl . -read /Users/_activitymonitor &>/dev/null; then
    echo "[*] Creating service user..."
    # Find next available UID >= 500
    UID=$(dscl . -list /Users UniqueID | awk '{print $2}' | sort -n | tail -1)
    UID=$((UID + 1))
    
    dscl . -create /Users/_activitymonitor
    dscl . -create /Users/_activitymonitor UserShell /usr/bin/false
    dscl . -create /Users/_activitymonitor RealName "ActivityMonitor Agent"
    dscl . -create /Users/_activitymonitor UniqueID "$UID"
    dscl . -create /Users/_activitymonitor PrimaryGroupID 20
    dscl . -create /Users/_activitymonitor NFSHomeDirectory "$DATA_DIR"
fi

# Set permissions
chown -R _activitymonitor:staff "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 750 "$CONFIG_DIR" "$LOG_DIR" "$DATA_DIR"
chmod 755 "$AGENT_PATH"
chmod 644 "$PLIST_PATH"
chown root:wheel "$PLIST_PATH"

# Load launchd service
echo "[*] Loading launchd service..."
launchctl load "$PLIST_PATH"

if launchctl list | grep -q com.activitymonitor.agent; then
    echo "[+] Service loaded successfully!"
else
    echo "[-] Failed to load service"
    exit 1
fi

echo
echo "========================================"
echo "Installation Complete"
echo "========================================"
echo "Service Name: $SERVICE_NAME"
echo "Binary Path: $AGENT_PATH"
echo "Plist Path: $PLIST_PATH"
echo "Config Dir: $CONFIG_DIR"
echo "Log Dir: $LOG_DIR"
echo "Data Dir: $DATA_DIR"
echo
echo "To manage the service:"
echo "  Start:   launchctl load $PLIST_PATH"
echo "  Stop:    launchctl unload $PLIST_PATH"
echo "  Status:  launchctl list com.activitymonitor.agent"
echo "  Logs:    tail -f $LOG_DIR/*.log"
echo
