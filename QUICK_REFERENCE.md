# 🔍 Quick Reference Card

**ActivityMonitor Enterprise v3.1.0** — Fast Lookup Guide

---

## 📍 I Need To...

### ...Get Started
- **First time?** → [START_HERE.md](START_HERE.md)
- **In a hurry?** → [QUICK_BUILD.md](QUICK_BUILD.md)
- **Find something?** → [INDEX.md](INDEX.md)

### ...Build & Deploy
```bash
# Agent (Rust)
cd agent && cargo build --release

# Server (Rust)
cd server && cargo build --release

# Dashboard (React)
cd dashboard && npm install && npm run build

# Docker (all services)
docker-compose up -d
```

### ...Understand the System
- **Architecture?** → [ARCHITECTURE.md](ARCHITECTURE.md)
- **Database schema?** → [ARCHITECTURE.md](ARCHITECTURE.md#database-schema)
- **How APIs work?** → [API_REFERENCE.md](API_REFERENCE.md)

### ...Configure Something
- **All settings?** → [API_REFERENCE.md#configuration](API_REFERENCE.md)
- **Environment vars?** → [QUICK_BUILD.md#environment-variables](QUICK_BUILD.md)
- **Database?** → [ARCHITECTURE.md#database](ARCHITECTURE.md)

### ...Fix a Problem
- **Agent won't compile?** → [AGENT_BUILD_SUMMARY.md](AGENT_BUILD_SUMMARY.md)
- **Can't connect to RabbitMQ?** → [API_REFERENCE.md#troubleshooting](API_REFERENCE.md)
- **Database error?** → [API_REFERENCE.md#troubleshooting](API_REFERENCE.md)
- **Dashboard not loading?** → [QUICK_BUILD.md#troubleshooting](QUICK_BUILD.md)

### ...Deploy on Windows
- **Step-by-step?** → [WINDOWS_DEMO_GUIDE.md](WINDOWS_DEMO_GUIDE.md)
- **Via Docker?** → [QUICK_BUILD.md](QUICK_BUILD.md)
- **Installer script?** → `deploy/install-windows.bat`

### ...Understand Features
- **Heatmaps & Protection?** → [HEATMAPS_AND_PROTECTION_GUIDE.md](HEATMAPS_AND_PROTECTION_GUIDE.md)
- **Real-time sync?** → [WEBSOCKET_ARCHITECTURE.md](WEBSOCKET_ARCHITECTURE.md)
- **What's new in v3.1.0?** → [CHANGELOG.md](CHANGELOG.md)

---

## 🎯 Command Quick Reference

### Build
```bash
# Agent
cd agent && cargo build --release

# Server
cd server && cargo build --release

# Dashboard
cd dashboard && npm run build

# All (from root)
cargo build --release -p agent -p server
cd dashboard && npm run build
```

### Run
```bash
# Docker services
docker-compose up -d

# Agent
./agent/target/release/activity-monitor-agent.exe

# Server
./server/target/release/activity-monitor-server

# Dashboard (dev)
cd dashboard && npm run dev

# Dashboard (prod)
npm run build && npm run preview
```

### Test
```bash
# Agent tests
cd agent && cargo test -- --nocapture

# Server tests
cd server && cargo test -- --nocapture

# Dashboard tests
cd dashboard && npm test

# Integration test
curl http://localhost:3000/api/health
```

### Deploy
```bash
# Windows (batch script)
deploy\install-windows.bat

# Linux/macOS
sudo bash deploy/install-linux.sh

# Via Docker
docker-compose up -d && ./agent/target/release/activity-monitor-agent.exe
```

---

## 📋 System Requirements

| Component | Requirement | Status |
|-----------|-------------|--------|
| **PostgreSQL** | 14+ with TimescaleDB | ✅ |
| **RabbitMQ** | 3.10+ | ✅ |
| **Redis** | 7+ | ✅ |
| **Rust** | 1.70+ | ✅ Agent compiled |
| **Node.js** | 18+ | ✅ Dashboard ready |
| **Docker** | Optional | ✅ Available |

---

## 🔑 Key Files & Locations

| File | Purpose | Location |
|------|---------|----------|
| **Agent binary** | Compiled executable | `agent/target/release/activity-monitor-agent.exe` |
| **Server binary** | API server | `server/target/release/activity-monitor-server` |
| **Database schema** | SQL migrations | `migrations/` |
| **Dashboard source** | React frontend | `dashboard/src/` |
| **Deployment scripts** | Install/setup | `deploy/` |
| **Docker config** | Container services | `docker-compose.yml` |

---

## ⚙️ Configuration

### Environment Variables (Copy to `.env`)
```bash
# Agent
RABBITMQ_URL=amqp://guest:guest@localhost:5672/
AGENT_OFFLINE_CACHE_KEY=dev-cache-key-change-in-production

# Server
SERVER_PORT=3000
DATABASE_URL=postgresql://monitor_user:password@localhost/activity_monitor
RABBITMQ_URL=amqp://guest:guest@localhost:5672/
JWT_SECRET=your-32-char-secret-key-here

# Features
HEATMAP_ENABLED=true
PROCESS_PROTECTION_ENABLED=true
USB_TRACKING_ENABLED=true
```

---

## 🧪 Testing Checklist

- [ ] Agent compiles without errors
- [ ] Docker services start: `docker-compose ps`
- [ ] Agent connects to RabbitMQ: Check logs
- [ ] Server API responds: `curl localhost:3000/api/health`
- [ ] Dashboard loads: Open `http://localhost:5173`
- [ ] Activity appears: Check within 30 seconds
- [ ] Heatmaps generated: Check after 1 hour

---

## 📞 Support Resources

| Issue | Resource |
|-------|----------|
| **Getting started** | [START_HERE.md](START_HERE.md) |
| **Build errors** | [AGENT_BUILD_SUMMARY.md](AGENT_BUILD_SUMMARY.md) |
| **API questions** | [API_REFERENCE.md](API_REFERENCE.md) |
| **System design** | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Troubleshooting** | [QUICK_BUILD.md](QUICK_BUILD.md) |
| **Windows demo** | [WINDOWS_DEMO_GUIDE.md](WINDOWS_DEMO_GUIDE.md) |
| **Navigation** | [INDEX.md](INDEX.md) |

---

## 🔗 Documentation Map

```
START_HERE.md
├─ QUICK_BUILD.md ─────────────────────┐
├─ ARCHITECTURE.md ────────────────────┤─→ [INDEX.md for full guide]
├─ API_REFERENCE.md ──────────────────┤
├─ CHANGELOG.md ───────────────────────┤
├─ AGENT_BUILD_SUMMARY.md ────────────┤
├─ SESSION_SUMMARY.md ─────────────────┤
├─ WINDOWS_DEMO_GUIDE.md ──────────────┤
├─ HEATMAPS_AND_PROTECTION_GUIDE.md ───┤
├─ WEBSOCKET_ARCHITECTURE.md ──────────┤
└─ BUILD_COMPLETE.md ──────────────────┘
```

---

## ✨ Feature Status

### v3.0 (Base Features)
- ✅ Process monitoring (2-second intervals)
- ✅ Window title capture
- ✅ USB device detection
- ✅ Software inventory
- ✅ Offline encrypted cache
- ✅ Real-time WebSocket

### v3.1.0 (New Features)
- ✅ Keyboard/mouse heatmaps
- ✅ Process protection (anti-kill)
- ✅ Termination alerts
- ✅ Device nicknames
- ✅ Multi-device dashboard

---

## 🚀 Production Readiness

| Aspect | Status |
|--------|--------|
| **Agent build** | ✅ Compiled, zero errors |
| **Code quality** | ✅ Production-ready |
| **Documentation** | ✅ Complete & organized |
| **Security** | ✅ Process protection enabled |
| **Testing** | ⏳ Integration tests pending |
| **Deployment** | ✅ Scripts available |

---

**Version**: 3.1.0 | **Last Updated**: 2026-04-01 | **Status**: ✅ Ready for Deployment
