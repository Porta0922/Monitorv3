# ActivityMonitor Enterprise v3 — Windows Demo Guide

**Complete Step-by-Step Guide for Testing on Windows**

This guide walks you through setting up and testing ActivityMonitor on a single Windows machine (for demonstration purposes).

---

## Prerequisites

Before starting, ensure you have:

- **Windows 10/11** (tested)
- **PostgreSQL 14+** installed locally
- **RabbitMQ 3.10+** running (or Docker)
- **Rust 1.70+** installed
- **Node.js 18+** installed
- **Administrator privileges** (for service installation)
- **Git** (for cloning)

### Quick Check
```powershell
# Open PowerShell and verify installations:
psql --version          # Should show PostgreSQL 14+
rabbitmqctl version     # Should work if RabbitMQ is running
rustc --version         # Should show Rust 1.70+
node --version          # Should show Node.js 18+
```

---

## Part 1: Setup Backend Infrastructure (15 minutes)

### Step 1.1: Create PostgreSQL Database

```powershell
# Open Command Prompt or PowerShell as Administrator

# Connect to PostgreSQL
psql -U postgres

# In psql, run:
CREATE USER monitor_user WITH PASSWORD 'password123';
CREATE DATABASE activity_monitor OWNER monitor_user;
\c activity_monitor
CREATE EXTENSION IF NOT EXISTS timescaledb;
\q
```

### Step 1.2: Apply Database Schema

```powershell
# From project root directory
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3

# Apply migrations
psql -U monitor_user -d activity_monitor -f migrations\001_init_schema.sql

# Verify (should show 7 tables)
psql -U monitor_user -d activity_monitor -c "\dt"
```

### Step 1.3: Start RabbitMQ

**Option A: Using Docker (Recommended)**
```powershell
# If Docker is installed
docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management

# Access management UI: http://localhost:15672 (guest/guest)
```

**Option B: Native Installation**
```powershell
# If RabbitMQ installed via Chocolatey
rabbitmq-service start

# Verify status
rabbitmqctl status
```

### Step 1.4: Configure Environment

```powershell
# Copy and edit .env
Copy-Item .env.example .env

# Edit .env with your values:
# DATABASE_URL=postgresql://monitor_user:password123@localhost:5432/activity_monitor
# RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F
# JWT_SECRET=your-random-32-char-key
# AES_KEY=0123456789abcdef0123456789abcdef
```

---

## Part 2: Build Server (10 minutes)

```powershell
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\server

# Build release binary
cargo build --release

# Should take 2-3 minutes
# Result: target\release\server.exe

# Start server (in new PowerShell window)
.\target\release\server.exe

# You should see:
# [INFO] Listening on 0.0.0.0:3000

# Keep this window open - don't close the server!
```

### Verify Server is Running

```powershell
# In another PowerShell window
curl http://localhost:3000/api/health

# Should return: {"status":"ok"}
```

---

## Part 3: Build Agent (10 minutes)

```powershell
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\agent

# Build release binary
cargo build --release

# Result: target\release\agent.exe
```

---

## Part 4: Build Dashboard (5 minutes)

```powershell
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\dashboard

# Install dependencies
npm install

# Build for production
npm run build

# Result: dist/ folder with production bundle
```

---

## Part 5: Deploy Agent as Service (10 minutes)

### Method A: Using Installer Script (Recommended)

```powershell
# As Administrator
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\deploy

# Run installer
.\install-windows.bat

# When prompted:
# "Enter device nickname (or press Enter for auto): [type name]"
# Example: "DEMO-PC-01" or "MyWorkstation"

# Press Enter and wait for:
# "[+] Service installed and started successfully!"
```

### Method B: Manual Installation (Advanced)

```powershell
# Create config directory
New-Item -ItemType Directory -Force -Path "$env:PROGRAMDATA\ActivityMonitor"

# Create .env file in that directory
@"
DEVICE_NICKNAME=Demo-PC-Windows
SERVER_URL=http://localhost:3000
RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F
"@ | Out-File "$env:PROGRAMDATA\ActivityMonitor\.env"

# Create service with NSSM (if installed)
nssm install ActivityMonitor "C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\target\release\agent.exe"
nssm start ActivityMonitor

# Verify
Get-Service -Name "ActivityMonitor" | Select-Object Status
```

### Verify Agent is Running

```powershell
# Check if service is running
Get-Service -Name "ActivityMonitor" | Select-Object Status

# Should show: "Running"

# Check logs
Get-Content "$env:PROGRAMDATA\ActivityMonitor\logs\output.log" -Tail 50
Get-Content "$env:PROGRAMDATA\ActivityMonitor\logs\error.log" -Tail 50
```

---

## Part 6: Test Dashboard (5 minutes)

### Option A: Development Server (Live Updates)

```powershell
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\dashboard

# Start dev server
npm run dev

# Visit: http://localhost:5173
```

### Option B: Production Build (Static Files)

```powershell
# Start simple HTTP server for dist folder
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3\dashboard

# Using Python (if installed)
python -m http.server 8000 --directory dist

# Or use Node.js
npx http-server dist -p 8000

# Visit: http://localhost:8000
```

---

## Part 7: Login & Verify (5 minutes)

### Step 7.1: Access Dashboard

Navigate to: **http://localhost:5173** (or 8000 if using production build)

You should see the **Login page**.

### Step 7.2: Create Admin User

First, add a user to the database:

```powershell
# Open PowerShell
psql -U monitor_user -d activity_monitor

# Generate Argon2id hash for password "demo123":
# You can use an online tool or PostgreSQL:
INSERT INTO users (username, password_hash, role) VALUES 
  ('admin', '$argon2id$v=19$m=19456,t=2,p=1$XXXX$XXXX...', 'admin');

# For demo, you can use a simple query:
INSERT INTO users (username, password_hash, role) VALUES
  ('admin', 'DEMO_HASH_REPLACE_ME', 'admin');
```

**Alternative: Use Server Registration Endpoint**

```powershell
# Once server is running, register a user
curl -X POST http://localhost:3000/api/register `
  -H "Content-Type: application/json" `
  -d '{"username":"admin","password":"demo123"}'

# Response should contain JWT token
```

### Step 7.3: Login

- **Username**: admin
- **Password**: demo123

After login, you should see a dashboard saying "No devices registered yet" (this is normal—agent needs 10-30 seconds to register).

---

## Part 8: Monitor Device Registration (10 minutes)

### Wait for Agent to Register

1. **Keep all processes running**:
   - ✅ PostgreSQL (running)
   - ✅ RabbitMQ (running)
   - ✅ Server (listening on :3000)
   - ✅ Agent (service running or in console)
   - ✅ Dashboard (open in browser)

2. **Wait 10-30 seconds** for agent to register

3. **Refresh dashboard** (F5) or click "🔄 Refresh" button

4. You should see a device card appear with:
   - Device nickname (e.g., "DEMO-PC-01")
   - MAC address
   - Status: 🟢 Online
   - Last seen: timestamp

### View Activity Logs

Click the **"📊 Activity"** tab to see:
- List of processes running
- Window titles
- Timestamps
- Duration

**Expected data**: Should show logs from the last 1-5 minutes with several entries per second.

### View Software Inventory

Click the **"📦 Inventory"** tab to see:
- Installed applications
- Versions
- Hash verification status

**Expected data**: Should show 50-200+ installed applications depending on your Windows system.

### View USB Events

Click the **"🔌 USB"** tab to see:
- USB device connections/disconnections
- Serial numbers
- Timestamps

**Action**: Try plugging/unplugging a USB device and refresh the page—you should see new events appear within 30 seconds.

### View Security Alerts

Click the **"🚨 Alerts"** tab to see:
- Hash mismatches (if any application hash changed)
- Suspicious applications
- Alert severity levels

**Expected**: Empty for demo (or may show alerts if hash changes detected).

---

## Part 9: Test Key Features (20 minutes)

### Test 1: Real-Time Updates

1. Open Activity page
2. Open Task Manager (Ctrl+Shift+Esc)
3. Launch a new application (e.g., Notepad)
4. Refresh dashboard Activity page
5. **Verify**: New process appears in logs within 2-5 seconds

### Test 2: Window Focus Tracking

1. Keep Activity page open
2. Switch between different applications
3. **Verify**: Window titles change in activity logs

### Test 3: USB Detection

1. Open USB Events page
2. Plug in a USB device
3. Wait 30 seconds and refresh
4. **Verify**: New USB event appears with device name/serial
5. Unplug device
6. **Verify**: "OUT" action appears in logs

### Test 4: Offline Resilience

1. **Stop RabbitMQ**:
   ```powershell
   # If using Docker
   docker stop rabbitmq
   
   # If native
   rabbitmq-service stop
   ```

2. **Keep agent running** (it will buffer data locally)

3. Perform actions:
   - Launch/close applications
   - Plug/unplug USB device
   - Keep agent running for 2-5 minutes

4. **Restart RabbitMQ**:
   ```powershell
   # If using Docker
   docker start rabbitmq
   
   # If native
   rabbitmq-service start
   ```

5. **Verify**: Agent syncs buffered data to server
6. Check dashboard—should show activity from offline period

### Test 5: Update Device Nickname

1. Go to Dashboard tab
2. Click "✎ Edit" on device card
3. Change nickname to something new (e.g., "Demo-Updated")
4. Click ✓ or press Enter
5. **Verify**: Device card updates with new nickname

---

## Part 10: Cleanup & Teardown (5 minutes)

When done testing:

```powershell
# Stop agent service
net stop ActivityMonitor

# OR if running in console, press Ctrl+C

# Stop server
# In server window, press Ctrl+C

# Stop dashboard
# In dashboard window, press Ctrl+C

# Stop RabbitMQ (optional)
docker stop rabbitmq
# OR
rabbitmq-service stop

# (Keep PostgreSQL running for next test)
```

### Optional: Uninstall Service

```powershell
# Remove agent service
nssm remove ActivityMonitor confirm

# OR
sc delete ActivityMonitor

# Clean up directory
Remove-Item -Recurse -Force "$env:PROGRAMDATA\ActivityMonitor"
```

---

## Troubleshooting During Demo

### Agent shows "Offline" in Dashboard

**Possible causes**:
1. Agent not running: `Get-Service -Name "ActivityMonitor"`
2. Server not running: `curl http://localhost:3000/api/health`
3. Database not accessible: `psql -U monitor_user -d activity_monitor`

**Solution**:
- Check logs: `$env:PROGRAMDATA\ActivityMonitor\logs\`
- Restart agent: `net start ActivityMonitor`
- Restart server (close and rerun)

### No Activity Logs Appearing

**Possible causes**:
1. Agent just started (takes 10-30 seconds to register)
2. RabbitMQ not running
3. Server logs not being saved

**Solution**:
- Wait 30 seconds and refresh
- Check RabbitMQ: `docker logs rabbitmq` or `rabbitmqctl status`
- Check server console for errors

### Dashboard Won't Load

**Possible causes**:
1. Dev server not started: `npm run dev` in dashboard directory
2. Port already in use: Check if :5173 is free
3. Backend server not responding

**Solution**:
- Start dev server: `cd dashboard && npm run dev`
- Check server health: `curl http://localhost:3000/api/health`
- Try different port: `npm run dev -- --port 5174`

### PostgreSQL Connection Fails

**Possible causes**:
1. PostgreSQL not running
2. Wrong connection string in .env
3. User/database doesn't exist

**Solution**:
```powershell
# Verify PostgreSQL running
pg_isready -h localhost

# Check connection
psql -U monitor_user -d activity_monitor -c "SELECT 1;"

# Update .env with correct credentials
# Restart server after changing .env
```

---

## Demo Talking Points

When presenting to stakeholders:

### Architecture
"ActivityMonitor is a three-tier system:
- **Agents** run on each client and capture process/window/USB data
- **Server** receives events via RabbitMQ and stores in TimescaleDB
- **Dashboard** queries the server and displays real-time information"

### Key Features to Highlight
1. **Real-time Monitoring**: "See activity within 2 seconds of it happening"
2. **Offline Resilience**: "Data buffers locally if server is down, syncs automatically"
3. **Cross-Platform**: "Works on Windows, Linux, macOS"
4. **USB Tracking**: "See exactly when external devices were connected"
5. **Security**: "All data encrypted, hash validation on executables"

### Performance
"With compression, we can store 1000 agents' data for 90 days in under 350GB."

### Scalability
"The system can handle 10,000+ events per second and scales horizontally."

---

## Next Steps After Demo

1. **Document Requirements**: Capture any feature requests or changes
2. **Test on Real Machines**: Deploy to 2-3 actual workstations
3. **Performance Testing**: Load test with 10+ concurrent agents
4. **Security Review**: Have IT/Security review encryption and authentication
5. **User Acceptance Testing (UAT)**: Get feedback from end users
6. **Deployment Planning**: Plan rollout schedule and communication

---

## Useful Demo Commands

```powershell
# Quick status check
$status = @{
    'PostgreSQL' = (Test-Connection localhost -Count 1 -ErrorAction Ignore)
    'RabbitMQ' = (Test-NetConnection localhost -Port 5672).TcpTestSucceeded
    'Server' = (curl http://localhost:3000/api/health -ErrorAction Ignore).StatusCode
    'Dashboard' = (curl http://localhost:5173 -ErrorAction Ignore).StatusCode
}
$status | Format-Table

# View agent logs in real-time
Get-Content "$env:PROGRAMDATA\ActivityMonitor\logs\output.log" -Tail 100 -Wait

# Query recent activity
psql -U monitor_user -d activity_monitor -c "SELECT device_id, app_name, window_title, timestamp FROM activity_logs ORDER BY timestamp DESC LIMIT 20;"

# Check device status
psql -U monitor_user -d activity_monitor -c "SELECT device_id, nickname, online, last_seen FROM devices;"
```

---

## Demo Duration

- **Quick Demo**: 10 minutes (just login and show UI)
- **Full Demo**: 30 minutes (all features + interaction)
- **Detailed Demo**: 60+ minutes (with Q&A + troubleshooting examples)

---

**Ready to demo! Good luck! 🚀**

For more details, see: `README.md`, `QUICK_START.md`, or `INDEX.md`
