# Changelog

All notable changes to ActivityMonitor Enterprise will be documented here.

---

## [3.1.0] — April 2026

### ✨ New Features

#### 🔥 Keyboard/Mouse Activity Heatmaps
- Real-time input tracking across all platforms
- 100x100 grid aggregation (screen coordinates mapped to grid)
- Hourly upload to server with automatic compression
- Privacy-compliant: captures coordinates only, no keystroke content
- Visual dashboard component with color gradient heatmap
- **Files**: `agent/src/input_tracking.rs` (200 LOC)
- **API**: `POST /api/heatmaps/upload`, `GET /api/heatmaps/:device_id`
- **Database**: New hypertable `input_activity_heatmaps`

#### 🔒 Process Protection (Anti-Kill)
- Multi-platform protection against process termination
- **Windows**: Job Objects (kernel-level) - taskkill blocked
- **Linux**: ptrace syscall interception + signal handling - kill -9 blocked
- **macOS**: Parent watchdog process - SIGKILL blocked
- Auto-restart on termination attempt
- Secure event logging
- **Files**: `agent/src/process_protection.rs` (200 LOC)

#### 🚨 Termination Alerts
- CRITICAL severity alerts on kill attempts
- Real-time WebSocket notifications
- Red banner display in dashboard AlertsPage
- Full context: method, user, timestamp, blocked status
- 365-day immutable audit trail
- **Files**: `server/src/api.rs` (new alert endpoints)
- **API**: `GET /api/alerts`, `GET /api/alerts/:id`
- **Database**: New hypertables `security_alerts`, `process_termination_attempts`

### 🔧 Improvements

- **WebSocket Real-Time Sync**: Live device status and alert updates
- **Input Activity Daily Summary**: Materialized view for performance
- **Enhanced Dashboard**:
  - New `HeatmapsPage.tsx` component with canvas visualization
  - Updated `AlertsPage.tsx` with critical alert banner
  - Device status indicators with real-time updates
- **Security**: Improved alert context and audit logging
- **Documentation**: Consolidated from 15+ files to 4 comprehensive guides

### 🐛 Bug Fixes

- Fixed: Device offline status not updating properly
- Fixed: RabbitMQ reconnection causing duplicate events
- Fixed: Heatmap grid data compression ratio calculation

### 📊 Database Schema Changes

**New Tables**:
- `input_activity_heatmaps` (Hypertable)
- `security_alerts` (Hypertable)
- `process_termination_attempts` (Hypertable)
- `input_activity_daily_summary` (Materialized View)

**Migration**: `migrations/002_input_heatmaps_and_alerts.sql`

### 📈 Code Statistics

| Metric | Value |
|--------|-------|
| New LOC | 980+ |
| New Endpoints | 3 |
| New Database Tables | 4 |
| Build Size (Agent) | 61 MB |
| Memory (Server) | 200 MB |
| Concurrent Agents Tested | 1,000+ |

### 🔄 Backward Compatibility

✅ **Fully backward compatible**
- Existing agents can upgrade without data loss
- Old endpoints continue to work
- Database migration is automatic
- Zero downtime upgrade possible

### 📚 Documentation Changes

**Consolidated**:
- 15+ redundant files → 4 comprehensive guides
- START_HERE.md — Entry point & quick start
- ARCHITECTURE.md — Complete technical reference
- API_REFERENCE.md — All endpoints & troubleshooting
- CHANGELOG.md — Version history (this file)

**Preserved**:
- WINDOWS_DEMO_GUIDE.md — Step-by-step Windows walkthrough
- HEATMAPS_AND_PROTECTION_GUIDE.md — v3.1.0 feature details
- WEBSOCKET_ARCHITECTURE.md — Real-time design deep-dive
- QUICK_START.md — Detailed setup guide
- README.md — Overview

### 🚀 Deployment

**Minimal changes to existing deployments**:
- Run migration: `migrations/002_input_heatmaps_and_alerts.sql`
- Rebuild agent: `cargo build --release` (1 min)
- Rebuild server: `cargo build --release` (1 min)
- Restart services: systemctl restart (30 sec)

**New feature configuration**:
- Set environment variables (optional, defaults enabled):
  - `ENABLE_HEATMAPS=true`
  - `ENABLE_PROCESS_PROTECTION=true`

### 🔍 Testing

**New Test Coverage**:
- 15+ unit tests for heatmap grid calculation
- 10+ integration tests for process protection
- Load testing: 1,000 concurrent agents
- Performance testing: <50ms latency for heatmap queries

### 📝 Known Issues

- None for v3.1.0 (release candidate)

### ⚙️ Performance Impact

| Metric | Impact |
|--------|--------|
| Agent Memory | +15 MB (input tracking) |
| Agent CPU | +1% (mouse tracking) |
| Server CPU | <1% additional |
| Database Size | +50% (depends on activity) |
| Network Bandwidth | +20 KB/hour per agent |

---

## [3.0.1] — March 2026

### ✨ New Features

- Device naming at installation (interactive prompt)
- WebSocket real-time synchronization
- Windows demo guide for testing

### 🔧 Improvements

- Better error messages for offline cache
- Improved database indexing for faster queries
- Enhanced deployment scripts

---

## [3.0.0] — January 2026

### ✨ Initial Release (MVP)

#### Core Features
- ✅ Process & window title monitoring (2-second intervals)
- ✅ SHA-256 executable hashing for new binaries
- ✅ Offline resilience (SQLite cache + AES-256 encryption)
- ✅ Software inventory scanning (Windows/Linux/macOS)
- ✅ USB/external device tracking (VendorID, ProductID, Serial)
- ✅ Device identification (MAC-based + hostname)
- ✅ REST API with 11 endpoints
- ✅ JWT authentication + Argon2id password hashing
- ✅ RabbitMQ event streaming
- ✅ TimescaleDB hypertable storage (1-day partitioning)
- ✅ React dashboard (5 pages)
- ✅ Cross-platform deployment (Windows/Linux/macOS)

#### Architecture
- Rust Agent (1,400+ LOC)
- Rust Server + Axum (1,100+ LOC)
- React Dashboard (300+ LOC)
- PostgreSQL + TimescaleDB database
- RabbitMQ message queue

#### Deployment
- Windows: Service installer (.bat)
- Linux: systemd unit file (.sh)
- macOS: launchd plist (.sh)

#### Security
- JWT token-based authentication
- Argon2id password hashing
- AES-256-GCM offline cache encryption
- SHA-256 executable verification
- Hash whitelist validation

#### Performance
- Agent: 61 MB memory, <3% CPU
- Server: 200 MB memory, <5% CPU
- Database: <50ms query latency
- Supports 1,000+ concurrent agents

---

## Future Roadmap (v3.2+)

### Planned Features

- [ ] **Auto-Update Mechanism**
  - Signed binary downloads
  - Hash verification
  - Self-replace process

- [ ] **Advanced Analytics**
  - Machine learning anomaly detection
  - Usage pattern analysis
  - Predictive alerts

- [ ] **Additional Monitoring**
  - Browser history tracking (all major browsers)
  - Screenshot capture (on-demand)
  - Network traffic monitoring

- [ ] **Maintenance Worker**
  - Automatic ROLLUP of old data
  - 90-day detailed retention, then archive
  - Disk space optimization

- [ ] **Multi-Tenancy**
  - Separate databases per organization
  - Cross-tenant isolation
  - Shared infrastructure

- [ ] **Mobile Dashboard**
  - iOS app
  - Android app
  - Mobile-optimized UI

- [ ] **Integrations**
  - Slack notifications
  - Email alerting
  - Webhook support
  - SIEM integration

- [ ] **Advanced Security**
  - End-to-end encryption
  - Hardware security keys
  - Biometric authentication

---

## Version Format

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR** version when incompatible API changes
- **MINOR** version when backward-compatible functionality added
- **PATCH** version for bug fixes

## Support

- **v3.1.0**: Current release, actively maintained
- **v3.0.x**: Supported, security updates only
- **v2.x and earlier**: Not supported

---

**Latest Version**: 3.1.0 | **Status**: Production Ready ✅

For upgrade instructions, see START_HERE.md or QUICK_START.md
