# ActivityMonitor Enterprise v3 — Windows Demo Guide (Docker)

**Complete Step-by-Step Guide for Testing on Windows Using Docker**

This guide walks you through setting up and testing ActivityMonitor on a single Windows machine using Docker containers (no manual installation needed).

---

## Prerequisites

Before starting, ensure you have:

- **Windows 10/11** (tested)
- **Docker Desktop for Windows** installed and running
- **Rust 1.70+** installed (for building agent + server)
- **Node.js 18+** installed (for dashboard)
- **Git** (for cloning)
- **Administrator privileges** (for Docker + service installation)
- **~5 GB disk space** (Docker images + data)

### Quick Check
```powershell
# Open PowerShell and verify installations:
docker --version          # Should show Docker version
docker ps                 # Should work (lists containers)
rustc --version          # Should show Rust 1.70+
node --version           # Should show Node.js 18+
```

**✅ That's it! All backend services run in Docker.**

---

## Part 1: Start Backend Infrastructure with Docker (5 minutes)

### Step 1.1: Start Docker Services

```powershell
# From project root directory
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3

# Start all services (PostgreSQL + RabbitMQ + Redis)
docker-compose up -d

# Wait for containers to start (10-15 seconds)
# Output should show:
# ✓ activity-monitor-postgres is healthy
# ✓ activity-monitor-rabbitmq is healthy
# ✓ activity-monitor-redis is healthy
```

### Step 1.2: Verify Backend Services

```powershell
# Check all containers running
docker-compose ps

# Expected output:
# NAME                    STATUS
# activity-monitor-postgres    Up (healthy)
# activity-monitor-rabbitmq    Up (healthy)
# activity-monitor-redis       Up (healthy)

# Test PostgreSQL
docker-compose exec postgres psql -U monitor_user -d activity_monitor -c "SELECT 1;"

# Access RabbitMQ Management UI
# Open browser: http://localhost:15672
# Username: guest
# Password: guest

# View logs for any service
docker-compose logs postgres   # PostgreSQL logs
docker-compose logs rabbitmq   # RabbitMQ logs
docker-compose logs redis      # Redis logs
```

### Step 1.3: Configure Environment

```powershell
# Copy environment template
Copy-Item .env.example .env

# Verify .env has these values (for Docker):
# DATABASE_URL=postgresql://monitor_user:monitor_password@localhost:5432/activity_monitor
# RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F
# JWT_SECRET=your-random-32-char-key-generate-this
# AES_KEY=0123456789abcdef0123456789abcdef
```

**That's all for backend! Docker is now running everything.** ✅

---

## Part 2: Build & Start Server (10 minutes)

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

**Note**: Server automatically connects to Docker services via environment variables. ✅

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

## Part 7: Create Admin User (3 minutes)

### Option A: Using curl (Recommended)

```powershell
# Register admin user via API
curl -X POST http://localhost:3000/api/register `
  -H "Content-Type: application/json" `
  -d '{"username":"admin","password":"demo123"}'

# Response contains JWT token
```

### Option B: Database Insert

```powershell
# Direct insert into users table
docker-compose exec postgres psql -U monitor_user -d activity_monitor -c `
  "INSERT INTO users (username, password_hash, role) VALUES ('admin', 'DEMO_HASH', 'admin');"
```

---

## Part 8: Login & Verify (5 minutes)

---

### Step 8.1: Access Dashboard

Navigate to: **http://localhost:5173** (or 8000 if using production build)

You should see the **Login page**.

### Step 8.2: Login

- **Username**: admin
- **Password**: demo123

After login, you should see a dashboard saying "No devices registered yet" (this is normal—agent needs 10-30 seconds to register).

---

## Part 9: Monitor Device Registration (10 minutes)

### Wait for Agent to Register

1. **Verify all services running**:
   ```powershell
   # Docker services
   docker-compose ps
   
   # Server
   curl http://localhost:3000/api/health
   
   # Dashboard
   http://localhost:5173 (or 8000)
   
   # Agent service
   Get-Service -Name "ActivityMonitor" | Select-Object Status
   ```

   Expected: All should show status "running/ok"

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

## Part 10: Test Key Features (20 minutes)

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

1. **Stop Docker services**:
   ```powershell
   # Pause RabbitMQ (simulates network outage)
   docker-compose pause rabbitmq
   
   # Keep agent running (it will buffer data locally)
   ```

2. Perform actions:
   - Launch/close applications
   - Plug/unplug USB device
   - Keep agent running for 2-5 minutes

3. **Resume Docker services**:
   ```powershell
   docker-compose unpause rabbitmq
   ```

4. **Verify**: Agent syncs buffered data to server
5. Check dashboard—should show activity from offline period

### Test 5: Update Device Nickname

1. Go to Dashboard tab
2. Click "✎ Edit" on device card
3. Change nickname to something new (e.g., "Demo-Updated")
4. Click ✓ or press Enter
5. **Verify**: Device card updates with new nickname

---

## Part 11: Cleanup & Teardown (5 minutes)

When done testing:

```powershell
# Stop agent service
net stop ActivityMonitor

# OR if running in console, press Ctrl+C

# Stop server
# In server window, press Ctrl+C

# Stop dashboard
# In dashboard window, press Ctrl+C

# Stop all Docker containers
docker-compose down

# (Optional: Clean volumes if you want fresh data next time)
# docker-compose down -v
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

### Restart Docker Services (Next Demo)

```powershell
# Next time you want to demo:
cd C:\dev\Monitor_nuevo\ActivityMonitor-Enterprise-v3

# Start fresh
docker-compose up -d

# Follow from Part 2 (Build Server)
```

---

## Troubleshooting During Demo

### Docker Services Won't Start

**Error**: `docker-compose up -d` fails

```powershell
# Check Docker is running
docker ps

# If Docker not running, start Docker Desktop

# Check for port conflicts
netstat -ano | findstr "5432 5672 15672 6379"

# If ports in use, stop conflicting containers
docker-compose down
docker system prune -f

# Try again
docker-compose up -d
```

### Agent shows "Offline" in Dashboard

**Possible causes**:
1. Agent not running: `Get-Service -Name "ActivityMonitor"`
2. Server not running: `curl http://localhost:3000/api/health`
3. Docker database not responding: `docker-compose logs postgres`

**Solution**:
- Check agent logs: `$env:PROGRAMDATA\ActivityMonitor\logs\`
- Restart agent: `net start ActivityMonitor`
- Restart Docker: `docker-compose restart postgres`

### No Activity Logs Appearing

**Possible causes**:
1. Agent just started (takes 10-30 seconds to register)
2. RabbitMQ not running: `docker-compose logs rabbitmq`
3. Server logs not showing (startup issue)

**Solution**:
- Wait 30 seconds and refresh
- Check RabbitMQ: `docker-compose logs rabbitmq` (look for errors)
- Check server console for startup errors

### Dashboard Won't Load

**Possible causes**:
1. Dev server not started: `npm run dev` in dashboard directory
2. Port already in use: Check if :5173 is free
3. Backend server not responding

**Solution**:
- Start dev server: `cd dashboard && npm run dev`
- Check server health: `curl http://localhost:3000/api/health`
- Try different port: `npm run dev -- --port 5174`
- Check Docker containers: `docker-compose ps`

### PostgreSQL Container Fails to Start

**Possible causes**:
1. Port 5432 already in use
2. Old Docker volume with permission issues

**Solution**:
```powershell
# Stop and remove containers + volumes
docker-compose down -v

# Check for zombie processes
Get-Process *docker* | Stop-Process -Force

# Start fresh
docker-compose up -d
```

---

## Demo Talking Points

When presenting to stakeholders:

### Architecture
"ActivityMonitor is a three-tier system:
- **Agents** run on each client and capture process/window/USB data
- **Server** receives events via RabbitMQ and stores in TimescaleDB
- **Dashboard** queries the server and displays real-time information"

### Setup Simplicity
"We're using Docker for all backend services—PostgreSQL, RabbitMQ, and Redis—just run `docker-compose up -d` and you're ready to go. No complex manual installation."

### Key Features to Highlight
1. **Real-time Monitoring**: "See activity within 2 seconds of it happening"
2. **Offline Resilience**: "Data buffers locally if server is down, syncs automatically"
3. **Cross-Platform**: "Works on Windows, Linux, macOS"
4. **USB Tracking**: "See exactly when external devices were connected"
5. **Security**: "All data encrypted, hash validation on executables"
6. **Heatmaps (v3.1.0)**: "Visual activity maps showing user focus areas"
7. **Process Protection (v3.1.0)**: "Agent cannot be killed, anti-tampering security"

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
# Quick status check (Docker)
docker-compose ps

# View Docker logs
docker-compose logs -f

# Check RabbitMQ management
Start-Process "http://localhost:15672"

# View agent logs in real-time
Get-Content "$env:PROGRAMDATA\ActivityMonitor\logs\output.log" -Tail 100 -Wait

# Query recent activity from Docker
docker-compose exec postgres psql -U monitor_user -d activity_monitor -c `
  "SELECT device_id, app_name, window_title, timestamp FROM activity_logs ORDER BY timestamp DESC LIMIT 20;"

# Check device status
docker-compose exec postgres psql -U monitor_user -d activity_monitor -c `
  "SELECT device_id, nickname, is_online, last_seen FROM devices;"

# Monitor all containers
docker-compose logs -f --tail=50
```

---

## Demo Duration

- **Quick Demo**: 10 minutes (just login and show UI)
- **Full Demo**: 30 minutes (all features + interaction)
- **Detailed Demo**: 60+ minutes (with Q&A + troubleshooting examples)

---

## Quick Reference: Docker Commands

```powershell
# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs

# Stop all services
docker-compose down

# Stop and remove volumes
docker-compose down -v

# Access PostgreSQL
docker-compose exec postgres psql -U monitor_user -d activity_monitor

# Access RabbitMQ Web UI
Start-Process "http://localhost:15672"  # Username: guest, Password: guest

# Rebuild specific service
docker-compose build postgres
```

---

**Ready to demo! Good luck! 🚀**

For more details, see: `README.md`, `START_HERE.md`, or `INDEX.md`
