# ActivityMonitor Enterprise v3.1.0 — System Architecture & Technical Reference

**Complete Technical Documentation**  
For Architects, Developers, DevOps Teams, and Advanced Users

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Component Architecture](#component-architecture)
3. [Data Flow & Messaging](#data-flow--messaging)
4. [Database Schema](#database-schema)
5. [API Specification](#api-specification)
6. [Deployment Architecture](#deployment-architecture)
7. [Security Design](#security-design)
8. [Performance & Scalability](#performance--scalability)
9. [Detailed Setup Guide](#detailed-setup-guide)
10. [Configuration Reference](#configuration-reference)

---

## System Overview

### Three-Tier Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ TIER 1: AGENTS (Client-Side Monitoring)                    │
│ - Windows/Linux/macOS Rust binaries                         │
│ - Process capture, USB tracking, input monitoring           │
│ - Local offline cache (SQLite + AES-256-GCM)               │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓ RabbitMQ (AMQP 0.9.1)
                     ↓ Topic Exchange: monitoring.*
                     │
┌────────────────────┴────────────────────────────────────────┐
│ TIER 2: SERVER (API + Orchestration)                       │
│ - Rust + Axum framework                                     │
│ - REST API: 11 endpoints                                   │
│ - WebSocket: real-time updates                             │
│ - JWT authentication + Argon2id password hashing           │
│ - RabbitMQ consumer: 3+ event types                        │
│ - Hash validation & security alert generation              │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ↓ PostgreSQL + TimescaleDB
                     ↓ 10 tables (8 hypertables)
                     │
┌────────────────────┴────────────────────────────────────────┐
│ TIER 3: FRONTEND (User Interface)                          │
│ - React 19 + TypeScript                                    │
│ - 7 pages + responsive design                              │
│ - JWT-based session management                             │
│ - Real-time WebSocket consumer                             │
└─────────────────────────────────────────────────────────────┘
```

### Component Roles

| Component | Technology | Purpose | Concurrency |
|-----------|-----------|---------|------------|
| **Agent** | Rust | Capture + offline buffer | 5+ tasks per machine |
| **Server** | Rust + Axum | REST API + RabbitMQ consumer | 1,000+ concurrent requests |
| **Database** | PostgreSQL 14+ | Time-series data + relational | Connection pooling (10 connections) |
| **Message Queue** | RabbitMQ 3.10+ | Event streaming | Topic exchange with 5+ queues |
| **Dashboard** | React 19 | Web UI | Single-page application |

---

## Component Architecture

### 1. Client Agent (Rust)

**Location**: `agent/` directory  
**Binary**: `target/release/agent` (Windows: `agent.exe`)  
**LOC**: 1,400+ | **Modules**: 8

#### Module Breakdown

```
agent/src/
├── main.rs                    (300 LOC)
│   └─ Orchestration loop: 5 concurrent tasks
│   └─ Device ID generation & nickname management
│   └─ Signal handling (Ctrl+C cleanup)
│   
├── monitoring.rs              (250 LOC)
│   └─ Process enumeration via sysinfo crate
│   └─ Window title capture (platform-specific)
│   └─ Interval: 2 seconds
│   └─ Output: activity_logs topic
│   
├── input_tracking.rs          (200 LOC) ← NEW v3.1.0
│   └─ Keyboard/mouse position tracking
│   └─ 100x100 grid aggregation
│   └─ Hourly heatmap upload
│   └─ Privacy: coordinates only, no keystrokes
│   
├── process_protection.rs      (200 LOC) ← NEW v3.1.0
│   └─ Windows: Job Objects (kernel-level)
│   └─ Linux: ptrace syscall interception
│   └─ macOS: Parent watchdog process
│   └─ Blocks: taskkill, kill -9, killall
│   
├── usb_detection.rs           (200 LOC)
│   └─ Windows: PowerShell Get-PnpDevice
│   └─ Linux: /sys/bus/usb scanning
│   └─ macOS: system_profiler enumeration
│   └─ Interval: 30 seconds
│   └─ Output: usb_queue topic
│   
├── inventory.rs               (250 LOC)
│   └─ Windows: Registry HKLM\Software
│   └─ Linux: /usr/bin + /opt enumeration
│   └─ macOS: /Applications bundle scanning
│   └─ Interval: 1 hour
│   └─ Output: inventory_queue topic
│   
├── offline_cache.rs           (200 LOC)
│   └─ SQLite database: local_cache.db
│   └─ AES-256-GCM encryption
│   └─ Auto-compression: zstd
│   └─ Max size: 500 MB
│   └─ FIFO sync on reconnect
│   
├── device_id.rs               (100 LOC)
│   └─ MAC address hash (SHA-256)
│   └─ Hostname inclusion
│   └─ Immutable identification
│   └─ Timezone-aware timestamps
│   
└── rabbitmq_publisher.rs      (200 LOC)
    └─ Async publish to topic exchange
    └─ Fallback to offline cache
    └─ Retry logic with exponential backoff
    └─ Connection pooling
```

#### Key Design Decisions

- **Language**: Rust → <100 MB binary, <3% CPU, cross-platform
- **Window Capture**: Native APIs (WinAPI, Xlib, Cocoa)
- **Encryption**: AES-256-GCM + random IV per write
- **Heatmaps**: Real-time grid updates, hourly aggregation
- **Protection**: Multi-layer approach (kernel + application level)

---

### 2. Server API (Rust + Axum)

**Location**: `server/` directory  
**Binary**: `target/release/server`  
**LOC**: 1,100+ | **Modules**: 7  
**Port**: 3000 (configurable)

#### Module Breakdown

```
server/src/
├── main.rs                    (150 LOC)
│   └─ Axum router initialization
│   └─ RabbitMQ consumer startup
│   └─ Database connection pool
│   └─ CORS/middleware configuration
│   
├── api.rs                     (350 LOC)
│   ├─ POST /api/register        (device registration)
│   ├─ POST /api/login           (user authentication)
│   ├─ GET  /api/devices         (list all agents)
│   ├─ GET  /api/devices/:id     (device details)
│   ├─ PUT  /api/devices/:id/nickname
│   ├─ POST /api/logs            (activity ingestion)
│   ├─ GET  /api/logs            (activity query)
│   ├─ POST /api/heatmaps/upload (NEW)
│   ├─ GET  /api/heatmaps/:id    (NEW)
│   ├─ GET  /api/alerts          (security alerts)
│   └─ GET  /api/health          (health check)
│   
├── auth.rs                    (150 LOC)
│   └─ JWT token generation
│   └─ Argon2id password hashing (time-memory hardened)
│   └─ Token validation middleware
│   └─ RBAC: admin, viewer roles
│   
├── db.rs                      (200 LOC)
│   └─ PostgreSQL connection pool (10 connections)
│   └─ Connection timeout: 5 seconds
│   └─ Query timeout: 30 seconds
│   └─ Transaction handling
│   └─ Error recovery
│   
├── rabbitmq_consumer.rs       (200 LOC)
│   └─ Topic subscription: activity_queue
│   └─ Topic subscription: usb_queue
│   └─ Topic subscription: inventory_queue
│   └─ Async event processing
│   └─ Automatic acknowledgment (after insert)
│   └─ Error handling: dead-letter queue
│   
├── whitelist.rs               (100 LOC)
│   └─ SHA-256 hash validation
│   └─ Whitelist lookup
│   └─ Alert generation on mismatch
│   └─ Cache validation results
│   
└── ws.rs                      (100 LOC) [v3.1.0+]
    └─ WebSocket connection management
    └─ Topic subscriptions
    └─ Message broadcasting
    └─ Auto-reconnect logic
```

#### API Endpoints (Complete Reference)

| Method | Endpoint | Auth | Purpose | Response |
|--------|----------|------|---------|----------|
| GET | `/api/health` | No | Health check | `{"status":"ok"}` |
| POST | `/api/register` | No | Device registration | `{"device_id":"..."}` |
| POST | `/api/login` | No | User authentication | `{"token":"...","expires":3600}` |
| GET | `/api/devices` | Yes | List devices | Array of device objects |
| GET | `/api/devices/:id` | Yes | Device details | Single device object |
| PUT | `/api/devices/:id/nickname` | Yes | Update nickname | Updated device object |
| POST | `/api/logs` | Yes | Submit activity logs | `{"recorded":42}` |
| GET | `/api/logs` | Yes | Query logs (time-range, device) | Array of log entries |
| POST | `/api/heatmaps/upload` | Yes | Upload heatmap grid (NEW) | `{"stored":true}` |
| GET | `/api/heatmaps/:id` | Yes | Get heatmap (NEW) | Heatmap object with grid data |
| GET | `/api/alerts` | Yes | Get alerts (filters: severity, resolved) | Array of alert objects |
| GET | `/api/alerts/:id` | Yes | Alert details | Single alert object |

#### Middleware Stack

```
Request → CORS → Authentication (JWT) → Logging → Handler → Response
                    ↓
                  Check token
                  Validate claims
                  Extract user_id
```

#### Error Responses

All errors return JSON with consistent format:

```json
{
  "error": "string",
  "details": "optional details",
  "code": "error_code"
}
```

Status codes:
- `200 OK` — Success
- `400 Bad Request` — Invalid input
- `401 Unauthorized` — Missing/invalid JWT
- `403 Forbidden` — Insufficient permissions
- `404 Not Found` — Resource doesn't exist
- `500 Internal Server Error` — Server error

---

### 3. Database (PostgreSQL + TimescaleDB)

**Location**: `migrations/` directory  
**Version**: PostgreSQL 14+, TimescaleDB 2.10+  
**Tables**: 10 (8 hypertables)  
**Indices**: 15+

#### Complete Schema

```sql
-- 1. DEVICES (Device Registry)
CREATE TABLE devices (
  device_id UUID PRIMARY KEY,
  nickname TEXT,
  hostname TEXT NOT NULL,
  mac_address TEXT NOT NULL UNIQUE,
  os_type TEXT, -- "windows", "linux", "macos"
  os_version TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_seen TIMESTAMPTZ DEFAULT NOW(),
  is_online BOOLEAN DEFAULT false,
  UNIQUE(hostname, mac_address)
);
CREATE INDEX idx_devices_last_seen ON devices(last_seen DESC);
CREATE INDEX idx_devices_nickname ON devices(nickname);

-- 2. ACTIVITY_LOGS (Hypertable, partitioned daily)
CREATE TABLE activity_logs (
  timestamp TIMESTAMPTZ NOT NULL,
  device_id UUID NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
  app_name TEXT NOT NULL,
  window_title TEXT,
  duration_seconds INT,
  is_active BOOLEAN,
  process_id INT,
  memory_mb FLOAT
);
SELECT create_hypertable('activity_logs', 'timestamp', 
  if_not_exists => TRUE, 
  chunk_time_interval => INTERVAL '1 day');
CREATE INDEX idx_activity_logs_device_time ON activity_logs(device_id, timestamp DESC);
SELECT set_chunk_time_interval('activity_logs', INTERVAL '1 day');
SELECT set_integer_now_func('activity_logs', 'now');

-- 3. USB_HISTORY (Hypertable, partitioned daily)
CREATE TABLE usb_history (
  timestamp TIMESTAMPTZ NOT NULL,
  device_id UUID NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
  action TEXT NOT NULL, -- "CONNECT", "DISCONNECT"
  hardware_id TEXT,
  vendor_id TEXT,
  product_id TEXT,
  serial_number TEXT,
  volume_label TEXT,
  capacity_mb BIGINT
);
SELECT create_hypertable('usb_history', 'timestamp', if_not_exists => TRUE);
CREATE INDEX idx_usb_history_device_time ON usb_history(device_id, timestamp DESC);

-- 4. APP_INVENTORY (Software Registry)
CREATE TABLE app_inventory (
  id SERIAL PRIMARY KEY,
  device_id UUID NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
  app_name TEXT NOT NULL,
  version TEXT,
  exe_hash TEXT NOT NULL, -- SHA-256
  verified BOOLEAN DEFAULT false,
  install_date DATE,
  scan_timestamp TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(device_id, exe_hash)
);
CREATE INDEX idx_app_inventory_device ON app_inventory(device_id);
CREATE INDEX idx_app_inventory_hash ON app_inventory(exe_hash);

-- 5. HASH_WHITELIST (Application Validation)
CREATE TABLE hash_whitelist (
  exe_hash TEXT PRIMARY KEY, -- SHA-256 (lowercase hex)
  app_name TEXT NOT NULL,
  description TEXT,
  is_trusted BOOLEAN DEFAULT true,
  added_at TIMESTAMPTZ DEFAULT NOW(),
  added_by UUID REFERENCES users(id)
);
CREATE INDEX idx_hash_whitelist_app ON hash_whitelist(app_name);

-- 6. INPUT_ACTIVITY_HEATMAPS (Hypertable, v3.1.0)
CREATE TABLE input_activity_heatmaps (
  timestamp TIMESTAMPTZ NOT NULL,
  device_id UUID NOT NULL REFERENCES devices(device_id),
  grid_data JSONB NOT NULL, -- 100x100 array
  screen_width INT,
  screen_height INT,
  mouse_moves INT,
  mouse_clicks INT,
  keyboard_events INT,
  compression_ratio FLOAT
);
SELECT create_hypertable('input_activity_heatmaps', 'timestamp', if_not_exists => TRUE);
CREATE INDEX idx_heatmaps_device_time ON input_activity_heatmaps(device_id, timestamp DESC);
SELECT set_chunk_time_interval('input_activity_heatmaps', INTERVAL '1 day');

-- 7. SECURITY_ALERTS (Alert Log, Hypertable, v3.1.0)
CREATE TABLE security_alerts (
  timestamp TIMESTAMPTZ NOT NULL,
  device_id UUID NOT NULL REFERENCES devices(device_id),
  severity TEXT NOT NULL, -- "CRITICAL", "HIGH", "MEDIUM", "LOW"
  alert_type TEXT NOT NULL, -- "HASH_MISMATCH", "TERMINATION_ATTEMPT", "USB_CONNECTED"
  message TEXT,
  context JSONB, -- Additional metadata
  resolved BOOLEAN DEFAULT false,
  resolved_at TIMESTAMPTZ
);
SELECT create_hypertable('security_alerts', 'timestamp', if_not_exists => TRUE);
CREATE INDEX idx_alerts_device_time ON security_alerts(device_id, timestamp DESC);
CREATE INDEX idx_alerts_severity ON security_alerts(severity);

-- 8. PROCESS_TERMINATION_ATTEMPTS (Hypertable, v3.1.0)
CREATE TABLE process_termination_attempts (
  timestamp TIMESTAMPTZ NOT NULL,
  device_id UUID NOT NULL REFERENCES devices(device_id),
  method TEXT NOT NULL, -- "TASKKILL", "KILL_9", "KILL_SIGNAL"
  user_name TEXT,
  blocked BOOLEAN DEFAULT true,
  details JSONB
);
SELECT create_hypertable('process_termination_attempts', 'timestamp', if_not_exists => TRUE);
CREATE INDEX idx_termination_device_time ON process_termination_attempts(device_id, timestamp DESC);

-- 9. INPUT_ACTIVITY_DAILY_SUMMARY (Materialized View)
CREATE TABLE input_activity_daily_summary (
  date DATE PRIMARY KEY,
  device_id UUID NOT NULL,
  total_mouse_moves INT,
  total_clicks INT,
  total_keyboard INT,
  peak_activity_hour INT,
  idle_duration_minutes INT
);
CREATE INDEX idx_daily_summary_device ON input_activity_daily_summary(device_id);

-- 10. USERS (Admin Accounts)
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username TEXT NOT NULL UNIQUE,
  password_hash TEXT NOT NULL, -- Argon2id
  role TEXT DEFAULT 'viewer', -- "admin", "viewer"
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_login TIMESTAMPTZ,
  is_active BOOLEAN DEFAULT true
);
CREATE INDEX idx_users_username ON users(username);
```

#### Hypertable Configuration

```sql
-- View hypertable info
SELECT * FROM timescaledb_information.hypertables;

-- Check chunk interval (should be 1 day)
SELECT chunk_interval FROM timescaledb_information.dimensions
WHERE hypertable_name = 'activity_logs';

-- View compression settings
SELECT * FROM timescaledb_information.compression_settings;

-- Enable compression for chunks > 7 days old
ALTER TABLE activity_logs SET (
  timescaledb.compress,
  timescaledb.compress_orderby = 'timestamp DESC, device_id'
);
SELECT compress_chunk(i) FROM show_chunks('activity_logs') i;
```

#### Data Retention Policy

| Table | Retention | Archive | Notes |
|-------|-----------|---------|-------|
| activity_logs | 90 days | Manual | ROLLUP after 30 days |
| usb_history | 30 days | Manual | Low volume |
| input_activity_heatmaps | 30 days | Manual | Compressed automatically |
| security_alerts | 365 days | Immutable | Audit trail |
| process_termination_attempts | 365 days | Immutable | Security audit |

---

### 4. Dashboard (React)

**Location**: `dashboard/` directory  
**Framework**: React 19 + TypeScript  
**LOC**: 400+ | **Pages**: 7 | **Components**: 15+

#### Page Structure

```
src/
├── App.tsx                    (Main router, authentication guard)
├── pages/
│   ├── LoginPage.tsx          (JWT login form)
│   │   └─ Username/password input
│   │   └─ Error handling
│   │   └─ Token storage in localStorage
│   │
│   ├── DashboardPage.tsx      (Device overview)
│   │   └─ Device list with status indicator
│   │   └─ Nickname editor
│   │   └─ Last seen timestamp
│   │   └─ Real-time status updates (WebSocket)
│   │
│   ├── ActivityPage.tsx       (Process timeline)
│   │   └─ Table with filtering (device, time range)
│   │   └─ App name, window title, duration
│   │   └─ Pagination (50 rows per page)
│   │   └─ Export to CSV
│   │
│   ├── InventoryPage.tsx      (Software audits)
│   │   └─ Application list with version
│   │   └─ Verification status (✓ trusted, ✗ unknown)
│   │   └─ Hash display (truncated)
│   │   └─ Scan timestamp
│   │
│   ├── USBPage.tsx            (Device timeline)
│   │   └─ Connection history
│   │   └─ Hardware IDs, serials
│   │   └─ Volume labels
│   │   └─ IN/OUT event timeline
│   │
│   ├── AlertsPage.tsx         (Security alerts, v3.1.0)
│   │   └─ RED BANNER: Process termination attempts
│   │   └─ Alert list with filters
│   │   └─ Severity color-coding
│   │   └─ Mark resolved / view context
│   │   └─ Real-time updates (WebSocket)
│   │
│   └── HeatmapsPage.tsx       (Activity visualization, v3.1.0)
│       └─ Device selector
│       └─ Date picker
│       └─ 100x100 grid visualization
│       └─ Color gradient (cool→hot)
│       └─ Stats: total clicks, movement, keyboard
│
├── components/
│   ├── NavBar.tsx             (Navigation)
│   ├── DeviceStatus.tsx       (Online/offline indicator)
│   ├── HeatmapVisualization.tsx (Canvas-based grid render)
│   ├── AlertBanner.tsx        (Critical alert notification)
│   └─ ... other components
│
├── api/
│   └── client.ts              (Axios HTTP client)
│       └─ JWT authorization header
│       └─ Error interceptor (401 → logout)
│       └─ WebSocket initialization
│
├── hooks/
│   ├── useAuth.ts             (Authentication state)
│   ├── useDevices.ts          (Device data fetching)
│   └── useAlerts.ts           (Alert subscription)
│
└── types/
    └── index.ts               (TypeScript interfaces)
```

#### Key Components

**LoginPage**:
```typescript
interface LoginRequest {
  username: string;
  password: string;
}

interface LoginResponse {
  token: string;
  expires_in: number;
  user: { id: string; username: string; role: string };
}
```

**AlertsPage (v3.1.0)**:
```typescript
interface Alert {
  id: string;
  device_id: string;
  timestamp: string;
  severity: "CRITICAL" | "HIGH" | "MEDIUM" | "LOW";
  alert_type: string;
  message: string;
  context: {
    method?: string;      // "TASKKILL", "KILL_9"
    user_name?: string;
    blocked?: boolean;
  };
  resolved: boolean;
}
```

**HeatmapsPage (v3.1.0)**:
```typescript
interface Heatmap {
  device_id: string;
  timestamp: string;
  grid_data: number[][];  // 100x100 grid (0-255 intensity)
  stats: {
    mouse_moves: number;
    mouse_clicks: number;
    keyboard_events: number;
  };
}

// Render: Canvas element with 100x100 cells, colored by intensity
```

---

## Data Flow & Messaging

### Activity Logging Flow (2-Second Cycle)

```
Agent: Process Monitor
   ↓
[Capture: every process, window title]
   ↓
Event Object:
{
  timestamp: "2026-04-01T14:35:22.123Z",
  device_id: "uuid...",
  app_name: "firefox.exe",
  window_title: "GitHub - Inbox",
  duration_seconds: 45,
  is_active: true,
  process_id: 1234,
  memory_mb: 256.5
}
   ↓
RabbitMQ Publish (activity_topic)
   ↓
Server: RabbitMQ Consumer
   ↓
[Validate & Normalize]
   ↓
PostgreSQL INSERT:
  activity_logs (timestamp, device_id, app_name, ...)
   ↓
Dashboard Query:
  GET /api/logs?device_id=X&limit=50
   ↓
[Display in ActivityPage table]
```

### USB Detection Flow (30-Second Cycle)

```
Agent: USB Detector
   ↓
[Scan: Windows PowerShell / Linux /sys/bus/usb / macOS system_profiler]
   ↓
Event Object:
{
  timestamp: "2026-04-01T14:35:22Z",
  device_id: "uuid...",
  action: "CONNECT",
  hardware_id: "VID_1234&PID_5678",
  vendor_id: "1234",
  product_id: "5678",
  serial_number: "ABC123XYZ",
  volume_label: "BACKUP_DRIVE"
}
   ↓
RabbitMQ Publish (usb_topic)
   ↓
Server: RabbitMQ Consumer
   ↓
PostgreSQL INSERT:
  usb_history (timestamp, device_id, action, ...)
   ↓
Dashboard Query:
  GET /api/usb?device_id=X
   ↓
[Display in USBPage timeline]
```

### Heatmap Flow (Hourly, v3.1.0)

```
Agent: Input Tracking
   ↓
[Real-time: mouse position, clicks, keyboard]
   ↓
[Aggregate: 100x100 grid (screen ÷ 100x100)]
   ↓
[Every hour: pack grid data]
   ↓
Heatmap Object:
{
  timestamp: "2026-04-01T14:00:00Z",
  device_id: "uuid...",
  grid_data: [[0,5,10,...], [2,8,15,...], ...],
  screen_width: 1920,
  screen_height: 1080,
  stats: {
    mouse_moves: 1250,
    mouse_clicks: 42,
    keyboard_events: 3800
  }
}
   ↓
POST /api/heatmaps/upload
   ↓
Server: Compress grid_data (JSONB)
   ↓
PostgreSQL INSERT:
  input_activity_heatmaps (timestamp, device_id, grid_data, ...)
   ↓
Dashboard: Fetch heatmap
  GET /api/heatmaps/:device_id?date=2026-04-01
   ↓
[Render 100x100 canvas with color gradient]
```

### Termination Alert Flow (v3.1.0)

```
Agent: Process Protection
   ↓
[Detect: taskkill / kill -9 / killall attempt]
   ↓
[Block: Job Objects / ptrace / parent watchdog]
   ↓
Alert Object:
{
  timestamp: "2026-04-01T14:35:22Z",
  device_id: "uuid...",
  severity: "CRITICAL",
  alert_type: "TERMINATION_ATTEMPT",
  message: "Process termination attempt blocked: taskkill",
  context: {
    method: "TASKKILL",
    user_name: "DOMAIN\\Administrator",
    blocked: true
  }
}
   ↓
POST /api/alerts (from agent or server detects abnormal restart)
   ↓
Server: Insert into security_alerts
   ↓
WebSocket Broadcast:
  All connected dashboards receive update
   ↓
Dashboard:
  [CRITICAL banner appears in red]
  AlertsPage shows alert with context
```

### Offline Sync Flow

```
Agent: RabbitMQ Offline
   ↓
[Queue events → offline_cache.db (SQLite, AES-256)]
   ↓
[Monitor: RabbitMQ connectivity]
   ↓
[Reconnected!]
   ↓
[Query: SELECT * FROM offline_cache ORDER BY timestamp ASC]
   ↓
[Publish: oldest events first (FIFO)]
   ↓
Server: Consumer receives
   ↓
[Deduplicate: by (timestamp, device_id, event_type)]
   ↓
[Insert: newer events into activity_logs]
   ↓
[Send: acknowledgment to agent]
   ↓
Agent: DELETE FROM offline_cache (older than 10 min)
   ↓
[Resume normal operation]
```

---

## API Specification

### Authentication

All endpoints (except `/health`, `/register`, `/login`) require:

```
Authorization: Bearer <JWT_TOKEN>
```

**JWT Claims**:
```json
{
  "sub": "user_id_uuid",
  "username": "admin",
  "role": "admin",
  "exp": 1234567890,
  "iat": 1234567890
}
```

**Token Expiry**: 24 hours (configurable)

### Request/Response Formats

All requests/responses are JSON:

```
Content-Type: application/json
```

### Complete Endpoint Documentation

#### 1. Health Check
```
GET /api/health
Response 200:
{
  "status": "ok",
  "uptime_seconds": 12345,
  "database": "connected",
  "rabbitmq": "connected"
}
```

#### 2. Device Registration
```
POST /api/register
Body:
{
  "device_id": "uuid",
  "hostname": "ubuntu-2024",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "os_type": "linux",
  "os_version": "22.04 LTS"
}
Response 200:
{
  "success": true,
  "device_id": "uuid",
  "message": "Device registered"
}
```

#### 3. User Login
```
POST /api/login
Body:
{
  "username": "admin",
  "password": "SecurePassword123"
}
Response 200:
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_in": 86400,
  "user": {
    "id": "uuid",
    "username": "admin",
    "role": "admin"
  }
}
Response 401:
{
  "error": "Invalid credentials"
}
```

#### 4. List Devices
```
GET /api/devices
Query: (optional)
  ?status=online
  ?nickname=my-laptop
Response 200:
[
  {
    "device_id": "uuid",
    "nickname": "my-laptop",
    "hostname": "LAPTOP-ABC123",
    "os_type": "windows",
    "last_seen": "2026-04-01T14:35:22Z",
    "is_online": true,
    "activity_count_1h": 360
  }
]
```

#### 5. Get Device Details
```
GET /api/devices/:device_id
Response 200:
{
  "device_id": "uuid",
  "nickname": "my-laptop",
  "hostname": "LAPTOP-ABC123",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "os_type": "windows",
  "os_version": "Windows 11 23H2",
  "created_at": "2026-03-01T00:00:00Z",
  "last_seen": "2026-04-01T14:35:22Z",
  "is_online": true
}
```

#### 6. Update Device Nickname
```
PUT /api/devices/:device_id/nickname
Body:
{
  "nickname": "john-laptop"
}
Response 200:
{
  "device_id": "uuid",
  "nickname": "john-laptop"
}
```

#### 7. Submit Activity Logs
```
POST /api/logs
Body:
{
  "device_id": "uuid",
  "logs": [
    {
      "timestamp": "2026-04-01T14:35:22Z",
      "app_name": "firefox.exe",
      "window_title": "GitHub",
      "duration_seconds": 45,
      "is_active": true,
      "process_id": 1234,
      "memory_mb": 256.5
    }
  ]
}
Response 200:
{
  "recorded": 1,
  "duplicates_skipped": 0
}
```

#### 8. Query Activity Logs
```
GET /api/logs
Query (required/optional):
  device_id=uuid
  ?app_name=firefox
  ?from=2026-04-01T00:00:00Z
  ?to=2026-04-01T23:59:59Z
  ?limit=50
  ?offset=0
Response 200:
[
  {
    "timestamp": "2026-04-01T14:35:22Z",
    "device_id": "uuid",
    "app_name": "firefox.exe",
    "window_title": "GitHub",
    "duration_seconds": 45,
    "is_active": true
  }
]
```

#### 9. Upload Heatmap (NEW v3.1.0)
```
POST /api/heatmaps/upload
Body:
{
  "device_id": "uuid",
  "timestamp": "2026-04-01T14:00:00Z",
  "grid_data": [[0,5,10,...], ...],
  "screen_width": 1920,
  "screen_height": 1080,
  "stats": {
    "mouse_moves": 1250,
    "mouse_clicks": 42,
    "keyboard_events": 3800
  }
}
Response 200:
{
  "stored": true,
  "grid_size": "100x100",
  "compression_ratio": 0.65
}
```

#### 10. Get Heatmap (NEW v3.1.0)
```
GET /api/heatmaps/:device_id
Query:
  ?date=2026-04-01
  ?hour=14
Response 200:
{
  "device_id": "uuid",
  "timestamp": "2026-04-01T14:00:00Z",
  "grid_data": [[0,5,10,...], ...],
  "stats": {
    "mouse_moves": 1250,
    "mouse_clicks": 42,
    "keyboard_events": 3800
  }
}
```

#### 11. Get Alerts (NEW v3.1.0)
```
GET /api/alerts
Query (optional):
  ?device_id=uuid
  ?severity=CRITICAL
  ?resolved=false
  ?from=2026-04-01T00:00:00Z
  ?limit=50
Response 200:
[
  {
    "id": "uuid",
    "timestamp": "2026-04-01T14:35:22Z",
    "device_id": "uuid",
    "severity": "CRITICAL",
    "alert_type": "TERMINATION_ATTEMPT",
    "message": "Process termination attempt blocked: taskkill",
    "context": {
      "method": "TASKKILL",
      "user_name": "DOMAIN\\Administrator",
      "blocked": true
    },
    "resolved": false
  }
]
```

---

## Deployment Architecture

### Single Server Setup

```
┌─ Development Machine ──────────────────────────────┐
│                                                    │
│ ┌─ PostgreSQL ──────────────────┐                │
│ │ Database + TimescaleDB        │                │
│ │ Port: 5432                    │                │
│ └──────────────────────────────┘                │
│           ↑                                      │
│           │ SQL connections                    │
│           │                                     │
│ ┌─ RabbitMQ ──────────────────┐                │
│ │ Message Broker              │                │
│ │ Port: 5672 (AMQP)           │                │
│ │ Port: 15672 (Management UI) │                │
│ └──────────────────────────────┘                │
│      ↑ & ↓                                       │
│  (publish/subscribe)                            │
│      ↑ & ↓                                       │
│ ┌─ Rust Server ──────────────────┐             │
│ │ Axum API (Port 3000)           │             │
│ │ ├─ REST handlers              │             │
│ │ ├─ RabbitMQ consumer          │             │
│ │ ├─ WebSocket endpoint         │             │
│ │ └─ JWT middleware             │             │
│ └──────────────────────────────┘             │
│      ↑                                         │
│      │ HTTP / WebSocket                       │
│      │                                         │
│ ┌─ React Dashboard ─────────────────┐        │
│ │ Port: 5173 (dev) or 3000 (prod)  │        │
│ │ ├─ Device management            │        │
│ │ ├─ Activity timeline            │        │
│ │ ├─ Heatmaps (NEW)              │        │
│ │ ├─ Alerts (NEW)                │        │
│ └──────────────────────────────────┘        │
│                                              │
│ ┌─ Client Machines (multiple) ──────────────┐
│ │ Windows/Linux/macOS                       │
│ │ ├─ Rust Agent (local)                    │
│ │ ├─ sqlite cache.db (local)               │
│ │ │   └─ AES-256 encrypted                │
│ │ └─ Sends to RabbitMQ (port 5672)       │
│ └──────────────────────────────────────────┘
│
└────────────────────────────────────────────────┘
```

### Multi-Server Setup (Production)

```
┌─ Load Balancer (nginx/HAProxy) ────────────────┐
│ Distribute: Round-robin                        │
│ SSL Termination                                │
│ WebSocket support                              │
└──────────────────┬───────────────────────────┘
                   ↓
    ┌──────────────┼──────────────┐
    ↓              ↓              ↓
┌─ Server 1   ┌─ Server 2   ┌─ Server 3
│ Axum (3000) │ Axum (3000) │ Axum (3000)
└──────┬──────┴──────┬──────┴────────┬──
       └──────────┬──────────────────┘
                  ↓
         ┌─ PostgreSQL ──────────────────┐
         │ Primary + Replicas            │
         │ Connection Pool: 50           │
         │ Read-Only Replicas: 2         │
         └──────────────────────────────┘
         
         ┌─ RabbitMQ Cluster ────────────┐
         │ 3+ nodes (High Availability)  │
         │ Mirrored queues               │
         └──────────────────────────────┘
         
         ┌─ Dashboard (CDN) ─────────────┐
         │ Static React build            │
         │ CloudFront / Cloudflare       │
         └──────────────────────────────┘
```

---

## Security Design

### Encryption Layers

#### 1. Offline Cache (Agent)
```
Raw Event Data
    ↓
[AES-256-GCM Encryption]
  Key: 32-byte hex
  IV:  16 random bytes per write
  Tag: 16-byte authentication
    ↓
Encrypted Blob (SQLite)
    ↓
Saved to: local_cache.db
```

#### 2. Network Communication
```
Agent → Server:
  ├─ RabbitMQ: TLS (configurable)
  └─ HTTP: HTTPS in production

Dashboard ← Server:
  ├─ HTTP: HTTPS in production
  └─ WebSocket: WSS (Secure WebSocket)
```

#### 3. Database
```
PostgreSQL:
  ├─ Network: SSL connection string
  ├─ Data at Rest: Filesystem encryption (OS-level)
  └─ Access Control: User roles + row-level security
```

### Authentication & Authorization

```
User Login:
  1. POST /api/login (username + password)
  2. Server: Hash password with Argon2id (time/memory hardened)
  3. Compare hash with stored hash
  4. On success: Generate JWT token
  5. Return: Token + 24-hour expiry
  
API Request:
  1. Client: Attach token in Authorization header
  2. Server: Verify JWT signature (using JWT_SECRET)
  3. Extract claims: user_id, role, exp
  4. Check expiry
  5. Allow/deny based on role

Roles:
  - admin: Full access (all endpoints)
  - viewer: Read-only (GET endpoints only)
```

### Hash Whitelist Validation

```
App Discovered:
  1. Calculate SHA-256 hash of executable
  2. Query hash_whitelist table
  3. If trusted: No action
  4. If unknown: Generate MEDIUM severity alert
  5. If malicious: Generate HIGH severity alert

Whitelist Maintenance:
  - Manual curation by admins
  - Include: Known-good app hashes
  - Exclude: Suspicious executables
  - Immutable audit trail
```

### Process Protection (v3.1.0)

#### Windows (Job Objects)
```
Process Tree:
  Agent.exe (parent)
    └─ Job Object (kernel-level restriction)
       └─ Cannot be terminated via taskkill
       └─ Auto-respawn child if killed
```

#### Linux (ptrace)
```
Protection Mechanism:
  Agent (main process)
    └─ Registers: ptrace_scope = 2 (restricted)
    └─ Blocks: kill -9, SIGKILL
    └─ Allows: SIGTERM (for graceful shutdown)
    └─ Parent watchdog: Restarts if killed
```

#### macOS (Parent Watchdog)
```
Process Monitoring:
  Watchdog process
    └─ Monitors: Agent process PID
    └─ On death: Restart agent
    └─ Reports: Termination to server
```

---

## Performance & Scalability

### Benchmarks

| Metric | Value | Notes |
|--------|-------|-------|
| Agent Memory | 61 MB | Stable, no leaks |
| Agent CPU | <3% | Background operation |
| Agent Disk I/O | Minimal | Async writes |
| Server CPU (100 agents) | <5% | Efficient Tokio handling |
| Server Memory (100 agents) | 200 MB | Connection pooling |
| DB Query Time (avg) | <50ms | Indexed queries |
| Dashboard Load | <1s | React optimized |
| WebSocket Latency | <500ms | Real-time updates |

### Scalability Limits (Single Server)

| Component | Limit | Bottleneck |
|-----------|-------|-----------|
| Concurrent Agents | 1,000+ | DB connection pool (50) |
| Events/Second | 5,000+ | RabbitMQ throughput |
| Dashboard Users | 100+ | WebSocket connections |
| Data Retention | 90 days | Disk space (~1 TB for 100 agents) |

### Optimization Strategies

#### 1. Database
```sql
-- Compression for activity_logs
ALTER TABLE activity_logs SET (timescaledb.compress);

-- Archive old data monthly
CREATE TABLE activity_logs_archive AS
SELECT * FROM activity_logs 
WHERE timestamp < NOW() - INTERVAL '90 days';

-- Analyze for query planner
ANALYZE activity_logs;

-- View table size
SELECT pg_size_pretty(pg_total_relation_size('activity_logs'));
```

#### 2. RabbitMQ
```
Configuration:
  memory_high_watermark: 0.6  (pause publishing at 60% memory)
  vm_memory_high_watermark_paging_ratio: 0.75
  
Queue Strategy:
  - activity_queue: TTL = 1 day (auto-expire old messages)
  - usb_queue: TTL = 7 days
  - inventory_queue: TTL = 30 days
```

#### 3. Server
```rust
// Connection pooling
let pool = PgPoolOptions::new()
  .max_connections(50)
  .connect(DATABASE_URL)
  .await?;

// Query caching
let mut cache = lru::LruCache::new(10000);

// Async handling
let listener = TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app).await?;
```

---

## Detailed Setup Guide

### Prerequisites Installation

#### Windows
```powershell
# 1. Install Rust
Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
.\rustup-init.exe -y

# 2. Install PostgreSQL + TimescaleDB
choco install postgresql15 pgadmin4
# Run pgAdmin → Services → Install TimescaleDB extension

# 3. Install RabbitMQ
choco install rabbitmq

# 4. Install Node.js
choco install nodejs

# 5. Verify
rustc --version
node --version
psql --version
```

#### Linux (Ubuntu 22.04)
```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 2. Install PostgreSQL + TimescaleDB
sudo apt update
sudo apt install postgresql-15 postgresql-contrib-15
sudo apt install timescaledb-2-postgresql-15

# 3. Install RabbitMQ
sudo apt install rabbitmq-server

# 4. Install Node.js
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt install nodejs

# 5. Verify
rustc --version
node --version
psql --version
sudo systemctl status rabbitmq-server
```

#### macOS
```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install PostgreSQL + TimescaleDB
brew install postgresql@15
brew install timescaledb
brew services start postgresql@15

# 3. Install RabbitMQ
brew install rabbitmq
brew services start rabbitmq

# 4. Install Node.js
brew install node

# 5. Verify
rustc --version
node --version
psql --version
brew services list
```

### Step-by-Step Deployment

See **START_HERE.md** "30-Minute Quick Start" for the complete deployment walkthrough.

---

## Configuration Reference

### Environment Variables (.env)

```bash
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=info  # debug, info, warn, error

# Database
DATABASE_URL=postgresql://monitor_user:password@localhost:5432/activity_monitor
DB_POOL_SIZE=10
DB_QUERY_TIMEOUT_SECS=30

# RabbitMQ
RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F
RABBITMQ_PREFETCH_COUNT=10

# Security
JWT_SECRET=your-random-32-character-key-here  # Min 32 chars
JWT_EXPIRY_HOURS=24
AES_KEY=0123456789abcdef0123456789abcdef  # 32-char hex

# Features
ENABLE_USB_TRACKING=true
ENABLE_INVENTORY=true
ENABLE_HEATMAPS=true
ENABLE_PROCESS_PROTECTION=true
ENABLE_WEBSOCKET=true

# Agent (set during installation)
DEVICE_NICKNAME=my-workstation
AGENT_SERVER_URL=http://localhost:3000
AGENT_REGISTRY_KEY=HKLM:\Software\ActivityMonitor  # Windows only
```

### Configuration Files

#### Agent Config (Windows)
```
Path: %APPDATA%\ActivityMonitor\config.yml
```

#### Agent Config (Linux)
```
Path: /etc/activity-monitor/config.yml
```

#### Agent Config (macOS)
```
Path: /Library/Application Support/ActivityMonitor/config.yml
```

---

**This document is the complete technical reference for ActivityMonitor Enterprise v3.1.0.**

For quick setup, see **START_HERE.md**.  
For API details, see **API_REFERENCE.md**.  
For operational issues, see **API_REFERENCE.md** troubleshooting section.
