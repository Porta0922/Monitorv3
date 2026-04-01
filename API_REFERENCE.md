# API Reference & Operations Guide

**For API Developers, DevOps, and System Operators**

---

## Quick Index

- [REST Endpoints](#rest-endpoints) — All 12 API endpoints
- [Configuration](#configuration) — Environment setup
- [Troubleshooting](#troubleshooting) — Common issues & solutions
- [Monitoring](#monitoring) — Health checks, metrics
- [Testing](#testing) — How to test the system
- [Advanced Topics](#advanced-topics) — WebSocket, rate limiting

---

## REST Endpoints

### Authentication Required
All endpoints except `/health`, `/register`, `/login` require JWT Bearer token.

```
Authorization: Bearer <token>
```

### 1. Health Check (No Auth)
```
GET /api/health

Response 200 OK:
{
  "status": "ok",
  "uptime_seconds": 12345,
  "database": "connected",
  "rabbitmq": "connected"
}

Use: Verify server is running
Interval: Every 5 seconds from agent
```

### 2. Device Registration (No Auth)
```
POST /api/register

Request:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "hostname": "LAPTOP-ABC123",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "os_type": "windows|linux|macos",
  "os_version": "Windows 11 23H2"
}

Response 200:
{
  "success": true,
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Device registered successfully"
}

Response 409 Conflict:
{
  "error": "Device already registered"
}

Use: First-time agent connection
```

### 3. User Login (No Auth)
```
POST /api/login

Request:
{
  "username": "admin",
  "password": "SecurePassword123"
}

Response 200:
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in": 86400,
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "admin",
    "role": "admin"
  }
}

Response 401 Unauthorized:
{
  "error": "Invalid username or password"
}

Use: Dashboard login
Token format: JWT (RS256 signed)
Expires: 24 hours (configurable)
```

### 4. List Devices
```
GET /api/devices

Query Parameters (optional):
  ?status=online|offline
  ?nickname=my-laptop
  ?hostname=LAPTOP-ABC
  ?os_type=windows|linux|macos

Response 200:
[
  {
    "device_id": "550e8400-e29b-41d4-a716-446655440000",
    "nickname": "john-laptop",
    "hostname": "LAPTOP-ABC123",
    "mac_address": "aa:bb:cc:dd:ee:ff",
    "os_type": "windows",
    "os_version": "Windows 11 23H2",
    "created_at": "2026-03-01T00:00:00Z",
    "last_seen": "2026-04-01T14:35:22Z",
    "is_online": true,
    "activity_count_1h": 360
  },
  ...
]

Use: Dashboard device list
Auth: Required (admin or viewer role)
```

### 5. Get Device Details
```
GET /api/devices/:device_id

Response 200:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "john-laptop",
  "hostname": "LAPTOP-ABC123",
  "mac_address": "aa:bb:cc:dd:ee:ff",
  "os_type": "windows",
  "os_version": "Windows 11 23H2",
  "created_at": "2026-03-01T00:00:00Z",
  "last_seen": "2026-04-01T14:35:22Z",
  "is_online": true
}

Response 404 Not Found:
{
  "error": "Device not found"
}

Use: Device detail page in dashboard
Auth: Required
```

### 6. Update Device Nickname
```
PUT /api/devices/:device_id/nickname

Request:
{
  "nickname": "john-new-laptop"
}

Response 200:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "john-new-laptop"
}

Response 400 Bad Request:
{
  "error": "Nickname must be 1-50 characters"
}

Use: Change friendly name for device
Auth: Required (admin role)
Nickname: User-facing identifier, shown in dashboard
```

### 7. Submit Activity Logs
```
POST /api/logs

Request:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "logs": [
    {
      "timestamp": "2026-04-01T14:35:22.123Z",
      "app_name": "firefox.exe",
      "window_title": "GitHub - Inbox",
      "duration_seconds": 45,
      "is_active": true,
      "process_id": 1234,
      "memory_mb": 256.5
    },
    {
      "timestamp": "2026-04-01T14:35:24.456Z",
      "app_name": "VSCode.exe",
      "window_title": "project.ts",
      "duration_seconds": 2,
      "is_active": false,
      "process_id": 5678,
      "memory_mb": 512.1
    }
  ]
}

Response 200:
{
  "recorded": 2,
  "duplicates_skipped": 0,
  "errors": []
}

Response 400 Bad Request:
{
  "error": "Invalid log format",
  "details": "timestamp is required for each log"
}

Use: Agent publishes activity (via RabbitMQ + server consumes)
Auth: Required
Batch size: Max 1,000 logs per request
Duplicates: Skipped by (timestamp, device_id, app_name) tuple
```

### 8. Query Activity Logs
```
GET /api/logs

Query Parameters:
  device_id=uuid         (REQUIRED)
  ?app_name=firefox      (optional filter)
  ?window_title=GitHub   (optional filter)
  ?from=2026-04-01T00:00:00Z  (optional, ISO 8601)
  ?to=2026-04-01T23:59:59Z    (optional, ISO 8601)
  ?limit=50              (default 100, max 1000)
  ?offset=0              (pagination)
  ?order=DESC|ASC        (timestamp order)

Response 200:
{
  "total": 3600,
  "returned": 50,
  "has_more": true,
  "logs": [
    {
      "timestamp": "2026-04-01T14:35:22.123Z",
      "device_id": "uuid",
      "app_name": "firefox.exe",
      "window_title": "GitHub - Inbox",
      "duration_seconds": 45,
      "is_active": true,
      "memory_mb": 256.5
    },
    ...
  ]
}

Response 404 Not Found:
{
  "error": "Device not found"
}

Use: Activity timeline in dashboard
Auth: Required
Performance: Indexed on (device_id, timestamp DESC)
Default range: Last 24 hours if not specified
```

### 9. Upload Heatmap (NEW v3.1.0)
```
POST /api/heatmaps/upload

Request:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-04-01T14:00:00Z",
  "grid_data": [
    [0, 5, 10, 15, ..., 0],      // Row 0 (y=0)
    [2, 8, 20, 25, ..., 1],      // Row 1 (y=1)
    ...
    [1, 3, 7, 12, ..., 0]        // Row 99 (y=99)
  ],
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
  "compression_ratio": 0.65,
  "storage_bytes": 4096
}

Response 400 Bad Request:
{
  "error": "Invalid grid_data",
  "details": "Grid must be 100x100 matrix"
}

Use: Agent uploads hourly heatmap aggregation
Auth: Required
Grid data: 100x100 array, values 0-255 (intensity)
Compression: JSONB stored as compressed in PostgreSQL
Upload frequency: Hourly per agent
```

### 10. Get Heatmap (NEW v3.1.0)
```
GET /api/heatmaps/:device_id

Query Parameters:
  ?date=2026-04-01           (YYYY-MM-DD)
  ?hour=14                   (optional, 0-23)
  ?include_stats=true        (default true)

Response 200:
{
  "device_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-04-01T14:00:00Z",
  "grid_data": [
    [0, 5, 10, ...],
    [2, 8, 20, ...],
    ...
  ],
  "stats": {
    "mouse_moves": 1250,
    "mouse_clicks": 42,
    "keyboard_events": 3800,
    "peak_y": 45,
    "peak_x": 73,
    "active_cells": 2847
  }
}

Response 404 Not Found:
{
  "error": "No heatmap data for this date/hour"
}

Use: Dashboard heatmap visualization
Auth: Required
Response time: <100ms (indexed query)
Default: Latest available hour if specific hour not found
```

### 11. List Alerts (NEW v3.1.0)
```
GET /api/alerts

Query Parameters (all optional):
  ?device_id=uuid
  ?severity=CRITICAL|HIGH|MEDIUM|LOW
  ?alert_type=TERMINATION_ATTEMPT|HASH_MISMATCH|USB_CONNECTED
  ?resolved=true|false
  ?from=2026-04-01T00:00:00Z
  ?to=2026-04-01T23:59:59Z
  ?limit=50
  ?offset=0

Response 200:
{
  "total": 5,
  "returned": 5,
  "alerts": [
    {
      "id": "alert-uuid",
      "timestamp": "2026-04-01T14:35:22Z",
      "device_id": "device-uuid",
      "severity": "CRITICAL",
      "alert_type": "TERMINATION_ATTEMPT",
      "message": "Process termination attempt blocked: taskkill",
      "context": {
        "method": "TASKKILL",
        "user_name": "DOMAIN\\Administrator",
        "blocked": true,
        "reason": "Job object protection active"
      },
      "resolved": false,
      "resolved_at": null,
      "resolved_by": null
    },
    ...
  ]
}

Use: AlertsPage in dashboard, shows CRITICAL as red banner
Auth: Required
Filtering: All parameters are optional, use combination for specific queries
Retention: 365 days immutable
Severity levels:
  CRITICAL — Process kill attempt, security compromise
  HIGH — Hash mismatch, unauthorized app
  MEDIUM — USB device connected, unknown app
  LOW — Informational events
```

### 12. Get Alert Details
```
GET /api/alerts/:alert_id

Response 200:
{
  "id": "alert-uuid",
  "timestamp": "2026-04-01T14:35:22Z",
  "device_id": "device-uuid",
  "severity": "CRITICAL",
  "alert_type": "TERMINATION_ATTEMPT",
  "message": "Process termination attempt blocked: taskkill",
  "context": {
    "method": "TASKKILL",
    "user_name": "DOMAIN\\Administrator",
    "blocked": true,
    "reason": "Job object protection active"
  },
  "resolved": false,
  "resolved_at": null
}

Response 404 Not Found:
{
  "error": "Alert not found"
}

Use: Alert detail page or investigation
Auth: Required
Context varies by alert_type
```

---

## Configuration

### Required Environment Variables

```bash
# Core Services
SERVER_HOST=0.0.0.0                    # Bind address
SERVER_PORT=3000                       # API port
RUST_LOG=info                          # Log level

# Database
DATABASE_URL=postgresql://monitor_user:password@localhost:5432/activity_monitor
DB_POOL_SIZE=10                        # Connection pool
DB_QUERY_TIMEOUT_SECS=30               # Max query time

# Message Queue
RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2F
RABBITMQ_PREFETCH_COUNT=10             # Consumer prefetch

# Security
JWT_SECRET=your-32-character-secret-here    # Min 32 chars, random
JWT_EXPIRY_HOURS=24
AES_KEY=0123456789abcdef0123456789abcdef   # 32-char hex

# Features
ENABLE_USB_TRACKING=true
ENABLE_INVENTORY=true
ENABLE_HEATMAPS=true
ENABLE_PROCESS_PROTECTION=true
ENABLE_WEBSOCKET=true
```

### Generating Secrets

```bash
# Linux/macOS (OpenSSL)
openssl rand -hex 16    # 32-character hex key

# Windows (PowerShell)
[System.Convert]::ToHexString((1..16 | ForEach-Object { Get-Random -Maximum 256 }))

# Rust
use rand::Rng;
let secret: String = (0..16)
  .map(|_| format!("{:02x}", rand::random::<u8>()))
  .collect();
println!("{}", secret);  // Copy to .env
```

---

## Troubleshooting

### Agent Issues

#### 1. Agent Not Connecting to Server

**Symptoms**: Device not appearing in dashboard, no activity logs

**Diagnosis**:
```bash
# 1. Check agent process running
Windows: tasklist | findstr agent
Linux:   ps aux | grep activity-monitor-agent
macOS:   ps aux | grep activity-monitor-agent

# 2. Verify server is reachable
Windows: Test-NetConnection localhost -Port 3000
Linux:   nc -zv localhost 3000
macOS:   nc -zv localhost 3000

# 3. Check agent config
Windows: cat %APPDATA%\ActivityMonitor\config.yml
Linux:   cat /etc/activity-monitor/config.yml
macOS:   cat ~/Library/Application\ Support/ActivityMonitor/config.yml

# 4. Check agent logs
Windows: wevtlog query Application /f:Source=ActivityMonitor
Linux:   sudo journalctl -u activity-monitor-agent -f
macOS:   log stream --predicate 'process == "activity-monitor-agent"'
```

**Solutions**:
- ✓ Ensure SERVER_ADDR in config points to correct IP/hostname
- ✓ Verify firewall allows port 3000
- ✓ Check RabbitMQ is running: `sudo systemctl status rabbitmq-server`
- ✓ Restart agent: `sudo systemctl restart activity-monitor-agent`
- ✓ Check disk space: `df -h` (ensure >100 MB free)

#### 2. Offline Cache Growing Too Large

**Symptoms**: Agent slowing down, large `local_cache.db` file (>500 MB)

**Diagnosis**:
```bash
# Check cache file size
Windows: dir "%LOCALAPPDATA%\ActivityMonitor\local_cache.db"
Linux:   ls -lh ~/.local/share/activitymonitor/local_cache.db
macOS:   ls -lh ~/Library/Application\ Support/ActivityMonitor/local_cache.db

# Check SQLite size
sqlite3 local_cache.db "SELECT COUNT(*) FROM events;"
```

**Solutions**:
- ✓ Verify network connectivity to RabbitMQ: `telnet localhost 5672`
- ✓ Check RabbitMQ queues: Open http://localhost:15672 (guest:guest)
- ✓ Restart agent to trigger sync
- ✓ Manually flush cache: Delete `local_cache.db`, agent will recreate on next sync

---

### Server Issues

#### 1. Server Won't Start

**Error**: `Error: Address already in use`

**Solution**:
```bash
# Find process using port 3000
Windows: netstat -ano | findstr :3000
Linux:   lsof -i :3000
macOS:   lsof -i :3000

# Kill the process
Windows: taskkill /PID <PID> /F
Linux:   kill -9 <PID>
macOS:   kill -9 <PID>

# Restart server
cd server && cargo run --release
```

#### 2. Database Connection Fails

**Error**: `Error: Connection refused`

**Diagnosis**:
```bash
# Check PostgreSQL running
Windows: sc query postgresql-x64-15
Linux:   sudo systemctl status postgresql
macOS:   brew services list | grep postgresql

# Verify connection string
echo $DATABASE_URL

# Test direct connection
psql $DATABASE_URL -c "SELECT 1;"
```

**Solutions**:
- ✓ Start PostgreSQL: `sudo systemctl start postgresql`
- ✓ Verify DATABASE_URL is correct
- ✓ Check credentials in .env
- ✓ Ensure TimescaleDB extension is installed: `psql -c "CREATE EXTENSION timescaledb;"`

#### 3. RabbitMQ Not Connecting

**Error**: `Error: Connection refused on AMQP port 5672`

**Diagnosis**:
```bash
# Check RabbitMQ running
sudo rabbitmqctl status

# Check listening ports
Windows: netstat -ano | findstr 5672
Linux:   ss -tuln | grep 5672
macOS:   lsof -i :5672

# Verify RABBITMQ_URL
echo $RABBITMQ_URL
```

**Solutions**:
- ✓ Start RabbitMQ: `sudo systemctl start rabbitmq-server`
- ✓ Access management UI: http://localhost:15672 (guest:guest)
- ✓ Reset RabbitMQ: `sudo rabbitmqctl reset`
- ✓ Check credentials in RABBITMQ_URL

---

### Dashboard Issues

#### 1. Dashboard Won't Load

**Error**: Blank page or 404 not found

**Diagnosis**:
```bash
# Check server running
curl http://localhost:3000/api/health

# Check dashboard dev server
npm run dev

# Open browser console (F12)
# Check for CORS errors or failed requests

# Check network tab
# Are requests to /api going to http://localhost:3000?
```

**Solutions**:
- ✓ Ensure server is running: `cd server && cargo run --release`
- ✓ Check VITE_API_URL in dashboard/.env
- ✓ Clear browser cache: Ctrl+Shift+Delete
- ✓ Rebuild dashboard: `cd dashboard && npm run build`

#### 2. Login Fails with 401

**Error**: Invalid credentials message after entering username/password

**Diagnosis**:
```bash
# Check admin user exists
psql $DATABASE_URL -c "SELECT username, role FROM users;"

# Check user login attempt
# Look at server logs for authentication errors
```

**Solutions**:
- ✓ Create admin user: See QUICK_START.md Step 4
- ✓ Verify JWT_SECRET is set: `echo $JWT_SECRET`
- ✓ Clear localStorage: F12 → Application → Storage → Local Storage → Clear
- ✓ Try with default credentials: admin / SecurePassword123

#### 3. No Data Appearing in Dashboard

**Symptoms**: Device list empty, activity page blank

**Diagnosis**:
```bash
# Check devices registered
psql $DATABASE_URL -c "SELECT COUNT(*) FROM devices;"

# Check activity logs
psql $DATABASE_URL -c "SELECT COUNT(*) FROM activity_logs;"

# Check agent registration
curl http://localhost:3000/api/devices
```

**Solutions**:
- ✓ Wait 30 seconds after agent starts (registration window)
- ✓ Verify agent is running: `ps aux | grep activity-monitor-agent`
- ✓ Check agent logs for errors
- ✓ Manually register device:
  ```bash
  curl -X POST http://localhost:3000/api/register \
    -H "Content-Type: application/json" \
    -d '{
      "device_id": "test-uuid",
      "hostname": "test",
      "mac_address": "aa:bb:cc:dd:ee:ff",
      "os_type": "linux",
      "os_version": "22.04"
    }'
  ```

---

## Monitoring

### Health Check Endpoint

```bash
# Every 5 seconds from agent
curl http://localhost:3000/api/health

Response:
{
  "status": "ok",
  "uptime_seconds": 12345,
  "database": "connected",
  "rabbitmq": "connected"
}

Use: Verify all services operational
Frequency: Every 5 seconds from each agent
```

### Database Health

```bash
# Connection count
psql $DATABASE_URL -c "SELECT datname, count(*) FROM pg_stat_activity GROUP BY datname;"

# Disk usage
psql $DATABASE_URL -c "SELECT pg_size_pretty(pg_database_size(current_database()));"

# Table sizes
psql $DATABASE_URL -c "SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) 
  FROM pg_stat_user_tables ORDER BY pg_total_relation_size(relid) DESC;"

# Hypertable chunks
psql $DATABASE_URL -c "SELECT * FROM timescaledb_information.chunks LIMIT 10;"

# Query performance
EXPLAIN ANALYZE SELECT * FROM activity_logs WHERE device_id = 'uuid' LIMIT 10;
```

### RabbitMQ Health

```bash
# Management UI
http://localhost:15672 (guest:guest)
- Connections: Should match number of agents
- Queues: Check message counts
- Consumers: Should show 1 per queue

# Command line
rabbitmqctl status
rabbitmqctl list_connections
rabbitmqctl list_queues
rabbitmqctl list_consumers
```

### Server Metrics

```bash
# Server logs (debug mode)
RUST_LOG=debug cargo run --release

# Monitor memory/CPU
Windows: Task Manager
Linux:   top -p <PID>
macOS:   Activity Monitor

# Database queries
SELECT query, calls, mean_time FROM pg_stat_statements 
ORDER BY mean_time DESC LIMIT 10;
```

---

## Testing

### Unit Tests

```bash
# Agent tests
cd agent
cargo test --lib

# Server tests
cd server
cargo test --lib

# Dashboard tests (future)
cd dashboard
npm test
```

### Integration Tests

```bash
# 1. Start server
cd server && cargo run --release

# 2. Deploy agent
sudo bash deploy/install-linux.sh

# 3. Wait 30 seconds for registration

# 4. Check device exists
curl http://localhost:3000/api/devices

# 5. Check activity logs (wait 2+ seconds)
curl http://localhost:3000/api/logs?device_id=<uuid>

# 6. Open dashboard
http://localhost:5173
# Login: admin / SecurePassword123
# Should see device and activity
```

### Load Test

```bash
# Simulate 50 concurrent agents
# Each sends 1 activity log every 2 seconds

# Monitor metrics:
# - CPU: Should stay <5%
# - Memory: Should stay <200 MB
# - DB latency: Should stay <50ms
# - RabbitMQ queue: Should stay empty
```

### API Test

```bash
# Using curl (or Postman)
TOKEN=$(curl -X POST http://localhost:3000/api/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"SecurePassword123"}' \
  | jq -r '.token')

# Test protected endpoint
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/api/devices

# Test heatmap upload
curl -X POST http://localhost:3000/api/heatmaps/upload \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "device_id":"uuid",
    "timestamp":"2026-04-01T14:00:00Z",
    "grid_data":[[0,5,10,...],[2,8,15,...],...],
    "screen_width":1920,
    "screen_height":1080,
    "stats":{"mouse_moves":1000,"mouse_clicks":50,"keyboard_events":3000}
  }'
```

---

## Advanced Topics

### WebSocket Connection

```javascript
// Client-side (React)
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onopen = () => {
  // Subscribe to device status updates
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'devices'
  }));
  
  // Subscribe to alerts
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'alerts'
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.type === 'device_status') {
    console.log('Device update:', message.data);
  }
  if (message.type === 'alert') {
    console.log('CRITICAL ALERT:', message.data);
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket closed, reconnecting...');
  // Auto-reconnect after 5 seconds
  setTimeout(() => {
    // Reinitialize WebSocket
  }, 5000);
};
```

### Rate Limiting

```
Default: 100 requests per minute per IP
Exceeding limit returns: 429 Too Many Requests

Apply exponential backoff:
- Attempt 1: Wait 1 second
- Attempt 2: Wait 2 seconds
- Attempt 3: Wait 4 seconds
```

### Pagination

```
Query large result sets:
?limit=50&offset=0    # First 50 records
?limit=50&offset=50   # Next 50 records
?limit=50&offset=100  # And so on...

Max limit: 1000 records
Default limit: 100 records
```

---

**Last Updated**: April 2026 | **Version**: 3.1.0
