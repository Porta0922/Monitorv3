# ActivityMonitor Enterprise v3 — Quick Start Guide

Get ActivityMonitor Enterprise running in **less than 15 minutes**.

## Prerequisites Checklist

- [ ] **Rust 1.70+** installed (`rustup --version`)
- [ ] **PostgreSQL 14+** with TimescaleDB extension
- [ ] **RabbitMQ 3.10+** running (`rabbitmqctl status`)
- [ ] **Node.js 18+** installed (`node --version`)
- [ ] **Git** for cloning repository

### Platform-Specific
- **Windows**: NSSM installed, Administrator access
- **Linux**: systemd available, sudo access
- **macOS**: Xcode CLI tools installed (`xcode-select --install`)

---

## Step 1: Clone & Setup (2 min)

```bash
# Clone repository
git clone https://github.com/yourcompany/ActivityMonitor-Enterprise-v3.git
cd ActivityMonitor-Enterprise-v3

# Copy environment template
cp .env.example .env

# Edit .env with your configuration
nano .env  # or use your preferred editor
```

### Key .env Variables
```env
# Database
DATABASE_URL=postgresql://monitor_user:password@localhost:5432/activity_monitor

# Message Queue
RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F

# Security
JWT_SECRET=your-random-32-character-secret-key-here
AES_KEY=0123456789abcdef0123456789abcdef  # 32-char hex for encryption

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Agent
DEVICE_NICKNAME=my-workstation  # Optional, can be set in dashboard later
```

**Pro Tip**: Generate random secrets:
```bash
# Linux/macOS
openssl rand -hex 16  # 32-char key

# Windows PowerShell
[System.Convert]::ToHexString((1..16 | ForEach-Object { Get-Random -Maximum 256 }))
```

---

## Step 2: Setup PostgreSQL + TimescaleDB (3 min)

### Windows (with pgAdmin)
```powershell
# Open pgAdmin → right-click "Servers" → Register → New Server
# Hostname: localhost, Port: 5432, Username: postgres, Password: (your password)
# Then execute SQL in Query Tool:

CREATE USER monitor_user WITH PASSWORD 'your_secure_password';
CREATE DATABASE activity_monitor OWNER monitor_user;

-- Enable TimescaleDB extension
\c activity_monitor
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

### Linux
```bash
# Install TimescaleDB (if not already installed)
sudo apt update && sudo apt install timescaledb-postgresql-14

# Connect to PostgreSQL
sudo -u postgres psql

# In psql prompt:
CREATE USER monitor_user WITH PASSWORD 'your_secure_password';
CREATE DATABASE activity_monitor OWNER monitor_user;
\c activity_monitor
CREATE EXTENSION IF NOT EXISTS timescaledb;
\q
```

### macOS
```bash
# Install TimescaleDB via Homebrew
brew install timescaledb

# Start PostgreSQL
brew services start postgresql

# Connect and setup
psql -U postgres

# In psql:
CREATE USER monitor_user WITH PASSWORD 'your_secure_password';
CREATE DATABASE activity_monitor OWNER monitor_user;
\c activity_monitor
CREATE EXTENSION IF NOT EXISTS timescaledb;
\q
```

### Apply Database Schema
```bash
# Windows (Command Prompt)
psql -U monitor_user -d activity_monitor -f migrations\001_init_schema.sql

# Linux/macOS
psql -U monitor_user -d activity_monitor < migrations/001_init_schema.sql
```

**Verify**: 
```bash
psql -U monitor_user -d activity_monitor -c "\dt"
# Should show: devices, activity_logs, app_inventory, usb_history, app_whitelist, security_alerts, users
```

---

## Step 3: Start RabbitMQ (1 min)

### Windows (with NSSM)
```powershell
# If installed via Chocolatey:
rabbitmq-service start

# Or via WSL/Docker:
docker run -d -p 5672:5672 -p 15672:15672 rabbitmq:3-management
```

### Linux
```bash
sudo systemctl start rabbitmq-server
sudo systemctl enable rabbitmq-server  # Auto-start on reboot

# Verify
sudo rabbitmqctl status
```

### macOS
```bash
brew services start rabbitmq

# Verify
rabbitmqctl status
```

**Access Management UI**: http://localhost:15672 (guest:guest)

---

## Step 4: Build & Run Server (3 min)

```bash
cd server

# Build release binary
cargo build --release

# Start server (will print "Listening on 0.0.0.0:3000")
./target/release/server

# OR in debug mode (faster to build, slower runtime):
cargo run
```

**Test Server**:
```bash
# In new terminal
curl http://localhost:3000/api/health
# Should return: {"status":"ok"}
```

**Create Admin User** (do this in server terminal or separate shell):
```bash
# Option 1: Via psql directly
psql -U monitor_user -d activity_monitor

# Then execute:
INSERT INTO users (username, password_hash, role) VALUES
  ('admin', '$argon2id$v=19$m=19456,t=2,p=1$XXXX$XXXX...', 'admin');

# Option 2: Use curl (once server is running)
curl -X POST http://localhost:3000/api/register \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"SecurePassword123"}'
```

**Admin Credentials (for dashboard login)**:
- Username: `admin`
- Password: `SecurePassword123` (change immediately in production!)

---

## Step 5: Build & Deploy Agent (3 min)

### Windows (Administrator Required)
```powershell
cd agent

# Build release binary
cargo build --release

# Copy to deployment directory
mkdir "C:\Program Files\ActivityMonitor"
copy target\release\agent.exe "C:\Program Files\ActivityMonitor\"

# OR use installer
cd ..\deploy
# Right-click install-windows.bat → Run as administrator
```

**Verify Agent Running**:
```powershell
Get-Process -Name agent
# OR check service:
Get-Service -Name "ActivityMonitor Agent" | Select Status
```

### Linux
```bash
cd agent
cargo build --release

# Use installer (requires sudo)
sudo bash ../deploy/install-linux.sh

# Verify
sudo systemctl status activitymonitor-agent
# Should show: active (running)
```

### macOS
```bash
cd agent
cargo build --release

# Use installer (requires sudo)
sudo bash ../deploy/install-macos.sh

# Verify
launchctl list | grep activitymonitor
# Should show the service
```

**Check Offline Cache Created**:
```bash
# Windows: C:\Users\<YourUsername>\AppData\Local\ActivityMonitor\local_cache.db
# Linux: ~/.local/share/activitymonitor/local_cache.db
# macOS: ~/Library/Application Support/ActivityMonitor/local_cache.db

# Verify agent can reach server:
curl http://localhost:3000/api/health
```

---

## Step 6: Start Dashboard (2 min)

```bash
cd dashboard

# Install dependencies (first time only)
npm install

# Start dev server
npm run dev
# Dashboard available at http://localhost:5173
```

**Build for Production**:
```bash
npm run build
# Output in: dist/ directory
```

---

## Step 7: Login & Verify

### Access Dashboard
Open browser: **http://localhost:5173**

Login with credentials:
- **Username**: `admin`
- **Password**: `SecurePassword123`

### Verify Data Flow

**On Dashboard**:
1. Go to **Dashboard** tab → Should see your machine listed
2. Go to **Activity** tab → Should see process logs from last 5 minutes
3. Go to **Inventory** tab → Should see installed applications
4. Go to **USB** tab → (will show events when USB devices connected)
5. Go to **Alerts** tab → (empty initially—alerts trigger on app hash changes)

**Via CLI (Database Query)**:
```bash
psql -U monitor_user -d activity_monitor

# Check device registered
SELECT device_id, nickname, last_seen FROM devices LIMIT 1;

# Check activity logs (should have recent entries)
SELECT COUNT(*) FROM activity_logs WHERE timestamp > NOW() - INTERVAL '1 hour';

# Check software inventory
SELECT app_name, COUNT(*) FROM app_inventory GROUP BY app_name LIMIT 5;

# Check USB history
SELECT * FROM usb_history ORDER BY timestamp DESC LIMIT 5;
```

---

## Troubleshooting

### Agent Not Showing Up in Dashboard
```bash
# 1. Check server logs (should show registration message)
# Ctrl+C to stop server, re-run with: RUST_LOG=debug cargo run

# 2. Verify agent process running
# Windows: tasklist | findstr agent
# Linux: ps aux | grep agent
# macOS: ps aux | grep agent

# 3. Check local_cache.db exists and has size > 0
# Windows: dir "%LOCALAPPDATA%\ActivityMonitor"
# Linux: ls -lh ~/.local/share/activitymonitor/
# macOS: ls -lh ~/Library/Application\ Support/ActivityMonitor/

# 4. Check RabbitMQ connection
# Windows: telnet localhost 5672
# Linux/macOS: nc -zv localhost 5672
```

### Dashboard Shows "401 Unauthorized"
```bash
# 1. Verify admin user exists
psql -U monitor_user -d activity_monitor -c "SELECT * FROM users;"

# 2. Clear browser localStorage (Ctrl+Shift+Delete in most browsers)

# 3. Try login again with correct credentials
```

### RabbitMQ Not Connecting
```bash
# Check RabbitMQ is running
sudo systemctl status rabbitmq-server  # Linux

# Check listening on port 5672
netstat -an | findstr 5672  # Windows
netstat -tuln | grep 5672   # Linux
lsof -i :5672              # macOS

# Default credentials (change in production!)
# Username: guest
# Password: guest
# Virtual Host: /
```

### PostgreSQL Connection Error
```bash
# Test connection directly
psql -U monitor_user -d activity_monitor -c "\dt"

# If "Connection refused":
# Windows: Check PostgreSQL service is running (Services.msc)
# Linux: sudo systemctl start postgresql
# macOS: brew services start postgresql
```

---

## Next Steps

### 1. **Secure Your Installation**
- [ ] Change admin password (dashboard settings)
- [ ] Update JWT_SECRET in .env (random 32-char)
- [ ] Change RabbitMQ guest password
- [ ] Enable HTTPS on server (use Let's Encrypt)
- [ ] Document device nicknames in your IT system

### 2. **Add More Agents**
- [ ] Deploy agent binary to other machines
- [ ] Agent auto-registers when it connects to server
- [ ] Assign friendly nicknames in Dashboard

### 3. **Tune Monitoring Intervals** (in agent/src/main.rs)
- Process capture: `2 seconds` (adjustable for heavy load)
- USB detection: `30 seconds` (lower = more overhead)
- Software scan: `1 hour` (rare changes, minimal impact)

### 4. **Setup Alerts**
- Monitor security_alerts table for hash changes
- Integrate with email/Slack via webhook (future feature)

### 5. **Archive Old Data** (future maintenance worker)
- Activity logs: 90-day retention (configurable)
- USB history: 7-day retention (configurable)

---

## Architecture at a Glance

```
Your Machines (Windows/Linux/macOS)
    ↓ (Agent sends every 2 seconds)
RabbitMQ (message broker, survives agent disconnect)
    ↓ (Server consumes and validates)
PostgreSQL + TimescaleDB (time-series data storage)
    ↓ (Dashboard queries API)
Your Browser (React dashboard)
```

- **Agent Offline?** Events buffer locally (SQLite), sync when reconnected
- **Server Down?** Agent continues monitoring, catches up when server restarts
- **Database Full?** Old data auto-partitioned by date, can be archived

---

## Common Commands

### View Logs
```bash
# Server logs (if running in foreground)
# Agent logs (Windows Event Viewer, Linux journalctl, macOS Console.app)

# Database query
psql -U monitor_user -d activity_monitor -c "SELECT * FROM activity_logs ORDER BY timestamp DESC LIMIT 10;"
```

### Restart Services
```bash
# Windows
net stop "ActivityMonitor Agent" && net start "ActivityMonitor Agent"

# Linux
sudo systemctl restart activitymonitor-agent

# macOS
launchctl stop com.activitymonitor.agent && launchctl start com.activitymonitor.agent
```

### Uninstall
```bash
# Windows (Administrator)
net stop "ActivityMonitor Agent"
sc delete "ActivityMonitor Agent"
rmdir /s /q "C:\Program Files\ActivityMonitor"

# Linux
sudo systemctl stop activitymonitor-agent
sudo rm /etc/systemd/system/activitymonitor-agent.service
sudo rm -rf /opt/activitymonitor

# macOS
sudo launchctl stop com.activitymonitor.agent
sudo rm /Library/LaunchDaemons/com.activitymonitor.agent.plist
sudo rm -rf /Library/Application\ Support/ActivityMonitor
```

---

## For More Help

- **Architecture Details**: See `docs/ARCHITECTURE.md`
- **Full API Reference**: See `docs/API_REFERENCE.md`
- **Database Schema**: See `docs/DATABASE_SCHEMA.md`
- **Troubleshooting**: See `docs/TROUBLESHOOTING.md`
- **Deployment Options**: See `docs/DEPLOYMENT.md`

---

**Ready to go!** 🚀

After Step 7, you should have:
- ✅ Server running on `localhost:3000`
- ✅ Dashboard accessible at `localhost:5173`
- ✅ Agent(s) connected and reporting data
- ✅ Data visible in dashboard within 5 seconds

Questions? Check the docs or reach out to the engineering team.
