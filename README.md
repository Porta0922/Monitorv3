<<<<<<< HEAD
# ActivityMonitor Enterprise v3

**Enterprise-grade activity monitoring solution** with offline resilience, hardware telemetry, centralized audit, and cross-platform support (Windows/Linux/macOS).

## What is ActivityMonitor Enterprise?

A comprehensive three-tier monitoring system designed for IT departments, security teams, and enterprises needing detailed visibility into:

- 📊 **Application Usage**: Track what applications are running and how long
- 🪟 **Window Activity**: Monitor active windows and user focus
- 💾 **Software Inventory**: Comprehensive OS-specific software auditing
- 🔌 **USB/Hardware Events**: Track external device connections in real-time
- 🚨 **Security Alerts**: Automated detection of hash changes and suspicious applications
- 📱 **Device Management**: Central dashboard for managing agent-equipped machines
- 🔒 **Offline Resilience**: Works without internet—syncs automatically on reconnect
- ⚡ **Real-time Monitoring**: 2-second capture intervals with millisecond precision

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Agents (Windows/Linux/macOS) — Rust Binaries            │
│  ├─ Process & Window Capture (2s intervals)              │
│  ├─ Software Inventory Scanner (1h intervals)            │
│  ├─ USB Device Tracking (30s intervals) [NEW]            │
│  ├─ Offline Cache (SQLite + AES-GCM encryption)          │
│  └─ Auto-sync via RabbitMQ when reconnected              │
└──────────────┬───────────────────────────────────────────┘
               │ (RabbitMQ Topic: monitoring.*)
               ▼
┌──────────────────────────────────────────────────────────┐
│  Server (Rust + Axum Framework)                          │
│  ├─ REST API (11+ endpoints)                             │
│  ├─ JWT Authentication (Argon2id password hashing)       │
│  ├─ RabbitMQ Consumer (3+ event types)                   │
│  ├─ Hash Whitelist Validation                            │
│  └─ Security Alert Generation                            │
└──────────────┬───────────────────────────────────────────┘
               │ (SQL)
               ▼
┌──────────────────────────────────────────────────────────┐
│  PostgreSQL + TimescaleDB (7 Tables + Hypertables)       │
│  ├─ activity_logs (partitioned by day, 90-day retention) │
│  ├─ devices (agent registry with metadata)               │
│  ├─ app_inventory (OS-agnostic software registry)        │
│  ├─ usb_history (hardware telemetry, 7-day retention)    │
│  ├─ security_alerts (hash changes, suspicious apps)      │
│  ├─ app_whitelist (hash → app mapping)                   │
│  └─ users (admin accounts with role-based access)        │
└──────────────┬───────────────────────────────────────────┘
               │ (REST API + Authentication)
               ▼
┌──────────────────────────────────────────────────────────┐
│  Dashboard (React 19 + TypeScript)                       │
│  ├─ Device Management (online status, nicknames)         │
│  ├─ Activity Timeline (app usage, idle time)             │
│  ├─ Software Audit (inventory with verification)         │
│  ├─ USB Timeline (device connections, serial tracking)   │
│  ├─ Security Alerts (hash anomalies, suspicious apps)    │
│  └─ User Management (admin console)                      │
└──────────────────────────────────────────────────────────┘
```

## Project Structure

```
.
├── agent/                          # Rust Client Agent (1,400+ LOC)
│   ├── src/
│   │   ├── main.rs                 # Entry point (3 concurrent monitoring tasks)
│   │   ├── monitoring.rs           # Process & window capture via sysinfo
│   │   ├── usb_detection.rs        # USB/external device tracking [NEW]
│   │   ├── offline_cache.rs        # SQLite + AES-GCM encryption layer
│   │   ├── inventory.rs            # OS-specific software scanner
│   │   ├── device_id.rs            # MAC + hostname device identification
│   │   └── rabbitmq_publisher.rs   # Event publishing to RabbitMQ topics
│   └── Cargo.toml
│
├── server/                         # Rust API Server (1,100+ LOC)
│   ├── src/
│   │   ├── main.rs                 # Axum server initialization
│   │   ├── api.rs                  # REST endpoints (11 routes)
│   │   ├── auth.rs                 # JWT tokens + Argon2id hashing
│   │   ├── db.rs                   # Database connection pooling
│   │   ├── rabbitmq_consumer.rs    # Event listener (activity, USB, inventory)
│   │   └── whitelist.rs            # Hash validation & alert generation
│   └── Cargo.toml
│
├── dashboard/                      # React Dashboard (300+ LOC)
│   ├── src/
│   │   ├── App.tsx                 # Main router (BrowserRouter)
│   │   ├── main.tsx                # React entry point
│   │   ├── api/
│   │   │   └── client.ts           # Axios HTTP client + JWT auth
│   │   ├── hooks/
│   │   │   └── useAuth.ts          # Authentication state management
│   │   ├── pages/
│   │   │   ├── LoginPage.tsx       # JWT login form
│   │   │   ├── DashboardPage.tsx   # Device list & management
│   │   │   ├── ActivityPage.tsx    # Activity logs table
│   │   │   ├── InventoryPage.tsx   # Software inventory
│   │   │   ├── USBPage.tsx         # USB events timeline
│   │   │   └── AlertsPage.tsx      # Security alerts [NEW]
│   │   └── types/
│   │       └── index.ts            # TypeScript interfaces
│   ├── public/
│   ├── index.html
│   └── package.json
│
├── migrations/                     # SQL Schemas
│   └── 001_init_schema.sql         # All 7 tables + hypertables
│
├── deploy/                         # Installation & Deployment
│   ├── install-windows.bat         # Windows service installer
│   ├── install-linux.sh            # systemd unit + service
│   └── install-macos.sh            # launchd plist + service
│
├── docs/                           # Comprehensive Documentation
│   ├── QUICK_START.md              # 5-minute setup guide
│   ├── ARCHITECTURE.md             # System design deep-dive
│   ├── API_REFERENCE.md            # All 11+ endpoints documented
│   ├── DATABASE_SCHEMA.md          # Table definitions & indices
│   ├── DEPLOYMENT.md               # Installation guide for all OS
│   └── TROUBLESHOOTING.md          # Common issues & fixes
│
├── Cargo.toml                      # Workspace manifest
├── .env.example                    # Environment variable template
├── README.md                       # This file
└── IMPLEMENTATION_SUMMARY.md       # Code statistics & metrics
```

## Key Features

### 🔍 Agent Monitoring
- **Process Monitoring**: Captures every running process, executable path, and memory usage
- **Window Activity**: Tracks active window title every 2 seconds for focus detection
- **Software Inventory**: Platform-specific scanning:
  - Windows: Registry HKLM\Software enumeration + version detection
  - Linux: /usr/bin, /opt scanning with version parsing
  - macOS: /Applications enumeration with bundle version extraction
- **USB Detection** ⭐ **NEW**: Real-time tracking of external storage connections:
  - Windows: PowerShell Get-PnpDevice parsing
  - Linux: /sys/bus/usb device scanning
  - macOS: system_profiler hardware enumeration
- **Offline Mode**: Local SQLite cache with AES-GCM encryption—syncs on reconnect in FIFO order

### 🖥️ Server Capabilities
- **REST API**: 11+ endpoints for device registration, log submission, queries
- **JWT Auth**: Token-based authentication with Argon2id password hashing
- **RabbitMQ Integration**: Consumes events from agents in real-time
- **Hash Validation**: Compares executable hashes against whitelist; generates security alerts
- **Concurrent Processing**: Tokio-based async handling of multiple agents

### 📊 Dashboard
- **Device Management**: View agent status (online/offline), assign friendly nicknames
- **Activity Timeline**: Filter logs by device, app, time range; see idle periods
- **Software Inventory**: Comprehensive app list with verification status
- **USB Timeline**: Chronological view of device connections with serial numbers
- **Security Alerts**: Real-time alerts for hash changes and suspicious applications
- **Responsive Design**: Works on desktop and tablet browsers

### 🛡️ Security
- **Device Identification**: MAC address hash + hostname—immutable and privacy-respecting
- **Encryption**: AES-GCM for offline cache; TLS for server communication
- **Password Security**: Argon2id hashing with random salts
- **Hash Whitelist**: Curated list of known-good application hashes
- **Alert Generation**: Automatic triggers on hash mismatches or unwhitelisted apps

## MVP Scope

### ✅ Included
- ✅ Process & window title monitoring (2-second intervals)
- ✅ SHA-256 executable hashing for new binaries
- ✅ Offline resilience (SQLite cache + AES-GCM encryption)
- ✅ Software inventory (Windows registry, Linux /usr/bin, macOS /Applications)
- ✅ **USB/External device tracking** ⭐ NEW
- ✅ Device identification (MAC-based + hostname)
- ✅ REST API with JWT authentication (11 endpoints)
- ✅ RabbitMQ event streaming (3+ event types)
- ✅ TimescaleDB hypertable (1-day partitioning for activity, 7-day for USB)
- ✅ React dashboard (5 pages: device mgmt, activity, inventory, USB, alerts)
- ✅ Automated installers (Windows service, systemd, launchd)

### ❌ Future Enhancements (v3.1+)
- Auto-update mechanism (signed binary downloads)
- Maintenance worker (weekly rollups, 90-day purge)
- Real-time WebSocket updates
- Advanced analytics & ML-based anomaly detection
- Browser history tracking (opt-in)
- Keyboard/mouse activity heatmaps
- Screenshot capture (on-demand)

## Prerequisites

### For Server & Agent Development
- **Rust 1.70+** ([Install](https://rustup.rs/))
- **PostgreSQL 14+** with TimescaleDB extension ([Guide](https://docs.timescale.com/))
- **RabbitMQ 3.10+** (optional; HTTP fallback planned)
- **Git** for version control

### For Dashboard
- **Node.js 18+** ([Install](https://nodejs.org/))
- **npm 9+** or **yarn 4+**

### Platform-Specific

**Windows:**
- NSSM (Non-Sucking Service Manager) for service management
- Administrator privileges for installation
- PowerShell 5.1+ (for USB detection)

**Linux:**
- systemd for service management
- sudo or root access
- libudev-dev for USB support (optional)

**macOS:**
- Xcode Command Line Tools (`xcode-select --install`)
- sudo or root access
- system_profiler available (standard on all macOS versions)

## Quick Start

### 1. Clone Repository
```bash
git clone https://github.com/yourcompany/ActivityMonitor-Enterprise-v3.git
cd ActivityMonitor-Enterprise-v3
```

### 2. Configure Environment
```bash
cp .env.example .env
# Edit .env with your settings:
# - DATABASE_URL (PostgreSQL connection)
# - RABBITMQ_URL (RabbitMQ broker)
# - JWT_SECRET (random 32-char key)
# - AES_KEY (32-byte hex for agent encryption)
```

### 3. Setup Database
```bash
# Create PostgreSQL database
createuser monitor_user -P
createdb -O monitor_user activity_monitor

# Apply schema
psql -U monitor_user -d activity_monitor < migrations/001_init_schema.sql
```

### 4. Build & Run Server
```bash
cd server
cargo build --release
./target/release/server
# Server listens on http://localhost:3000
```

### 5. Build & Deploy Agent
```bash
cd ../agent
cargo build --release

# Windows (as Administrator)
cd ../deploy
install-windows.bat

# Linux
sudo ./deploy/install-linux.sh

# macOS
sudo ./deploy/install-macos.sh
```

### 6. Start Dashboard
```bash
cd ../dashboard
npm install
npm run dev
# Dashboard available at http://localhost:5173
# Login with credentials from server setup
```

See **[QUICK_START.md](./docs/QUICK_START.md)** for detailed step-by-step instructions.

## API Endpoints

All endpoints require JWT Bearer token in Authorization header.

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/register` | Device registration |
| POST | `/api/login` | User authentication |
| GET | `/api/devices` | List all agents |
| POST | `/api/logs` | Submit activity logs |
| GET | `/api/activity?device_id=X&limit=Y` | Query activity timeline |
| GET | `/api/software?device_id=X` | Get software inventory |
| GET | `/api/usb?device_id=X` | Get USB events |
| GET | `/api/alerts?resolved=false` | Get security alerts |
| PATCH | `/api/device/:id` | Update device (nickname) |
| POST | `/api/alerts/:id/resolve` | Mark alert as resolved |
| GET | `/api/health` | Server health check |

Full API documentation: **[API_REFERENCE.md](./docs/API_REFERENCE.md)**

## Database Schema

### Tables Overview
- **devices**: Agent registry with device_id, nickname, last_seen, status
- **activity_logs**: TimescaleDB hypertable (1-day partitioning) with process/window events
- **app_inventory**: Software list with version, install date, executable hash
- **usb_history**: TimescaleDB hypertable (7-day partitioning) for device connections
- **app_whitelist**: Curated hash-to-app mapping for security validation
- **security_alerts**: Generated alerts for hash mismatches or suspicious apps
- **users**: Admin accounts with Argon2id password hashes

See **[DATABASE_SCHEMA.md](./docs/DATABASE_SCHEMA.md)** for detailed column definitions and indices.

## Deployment

### Windows Service
```bash
cd deploy
install-windows.bat
# Creates: C:\Program Files\ActivityMonitor\agent.exe
# Service: "ActivityMonitor Agent" (auto-start)
```

### Linux (systemd)
```bash
sudo ./deploy/install-linux.sh
# Creates: /opt/activitymonitor/agent
# Service: systemctl start/stop/status activitymonitor-agent
```

### macOS (launchd)
```bash
sudo ./deploy/install-macos.sh
# Creates: /Library/Application Support/ActivityMonitor/agent
# Service: launchctl start/stop com.activitymonitor.agent
```

See **[DEPLOYMENT.md](./docs/DEPLOYMENT.md)** for advanced configuration and troubleshooting.

## Monitoring Intervals

| Task | Interval | Purpose |
|------|----------|---------|
| Process Capture | 2 seconds | Real-time activity tracking |
| Window Title | 2 seconds | Focus detection & user activity |
| Software Scan | 1 hour | Inventory updates (low overhead) |
| USB Detection | 30 seconds | Hardware telemetry (balanced) |
| RabbitMQ Sync | On-demand | FIFO sync when reconnected |

## Offline Behavior

When RabbitMQ/network unavailable:
1. Agent continues monitoring locally
2. Events buffered in encrypted SQLite (local_cache.db + AES-GCM)
3. Events tagged with timestamps and ordered by capture time
4. On reconnection, FIFO sync sends oldest events first
5. Server deduplicates by timestamp + device_id + event_type

Maximum offline storage: ~10,000 events (~50MB encrypted database)

## Security Considerations

- ✅ Device IDs are MAC-address hashes—immutable across reboots
- ✅ Offline cache encrypted with AES-GCM (128-bit authentication)
- ✅ Passwords hashed with Argon2id (time/memory hardened)
- ✅ JWT tokens with 24-hour expiration (configurable)
- ✅ HTTPS recommended for all deployments
- ⚠️ Hash whitelist requires regular audits (semi-manual process)
- ⚠️ USB tracking may trigger privacy concerns—document in company policy

## Code Statistics

| Component | Files | LOC | Tests |
|-----------|-------|-----|-------|
| Agent | 7 | 1,400+ | 15 |
| Server | 6 | 1,100+ | 12 |
| Dashboard | 8 | 300+ | 0 |
| Database | 1 | 400+ | — |
| **Total** | **22** | **3,200+** | **27+** |

See **[IMPLEMENTATION_SUMMARY.md](./IMPLEMENTATION_SUMMARY.md)** for detailed metrics.

## Development

### Running Tests
```bash
# Agent tests
cd agent && cargo test --all-features

# Server tests
cd ../server && cargo test

# Dashboard tests (future)
cd ../dashboard && npm test
```

### Building Documentation
```bash
# All docs are Markdown in /docs
# View with: cat docs/ARCHITECTURE.md
# Or open in editor: code docs/
```

### Setting Up Development Environment
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install PostgreSQL + TimescaleDB
# Windows: https://docs.timescale.com/install/latest/installation-windows/
# Linux: https://docs.timescale.com/install/latest/installation-linux/
# macOS: brew install timescaledb

# Install RabbitMQ
# Windows: choco install rabbitmq-server
# Linux: sudo apt install rabbitmq-server
# macOS: brew install rabbitmq
```

## Troubleshooting

### Agent Won't Connect to RabbitMQ
1. Verify RabbitMQ is running: `rabbitmqctl status`
2. Check RABBITMQ_URL in .env
3. Agent will fall back to offline mode—check local_cache.db size

### Dashboard Shows No Devices
1. Verify server is running: `curl http://localhost:3000/api/health`
2. Check agent has registered: `psql -c "SELECT * FROM devices;"`
3. Verify JWT token is valid (check browser console)

### Offline Cache Not Syncing
1. Check network connectivity: `ping rabbitmq-server`
2. Verify agent is running: `systemctl status activitymonitor-agent`
3. Check local_cache.db size: `du -h /path/to/local_cache.db`
4. Manually trigger sync: Restart agent or bounce RabbitMQ

See **[TROUBLESHOOTING.md](./docs/TROUBLESHOOTING.md)** for more issues.

## License

Proprietary—Internal Use Only

## Support

- 📧 Email: engineering@company.com
- 🐛 Issues: Create GitHub issue (internal)
- 📚 Wiki: See /docs folder

---

**Version**: 3.0.0 (MVP) | **Last Updated**: January 2025 | **Status**: Production Ready ✅
=======
# Monitorv3
>>>>>>> a4fe8ca8e95efcc081d870b3f3fd77cf57b67983
