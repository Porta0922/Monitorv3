# NEXT STEPS - Security Integration Roadmap (osquery + MITRE ATT&CK)

## Objective
Integrate osquery-based threat detection into ActivityMonitor and map detections to MITRE ATT&CK, while enabling security search and operations from the dashboard.

## Scope Included
- Agent module: `osquery_runner` (periodic query execution + JSON parsing + RabbitMQ publish)
- Server ingestion: new queue consumer for `monitoring.security`
- Persistence: `security_events` table
- API: search/list/summary endpoints for security events
- Dashboard: dedicated "Seguridad" tab with filters, summary cards, MITRE links, CSV export
- Additional requested capabilities:
  - Search events by specific date/range from dashboard
  - USB copy alerts
  - Search active users by MAC

---

## Feasibility Snapshot
- Technical feasibility: HIGH
- Integration risk: MEDIUM
- Operational noise risk (false positives/perf): MEDIUM-HIGH
- Main prerequisite: finalize backend alert/security endpoints (currently partial/TODO)

---

## Proposed Architecture Changes

### 1) Agent (Rust)
Create `agent/src/osquery_runner.rs`:
- Check osquery availability (`osqueryi` or `osqueryd`) and fail silently if missing.
- Load enabled query packs and intervals from config.
- Run selected packs periodically.
- Parse JSON results into normalized security events.
- Publish to `monitoring.security` with existing event envelope pattern.

Suggested event payload fields:
- `timestamp`
- `device_id`
- `query_name`
- `query_pack`
- `mitre_techniques` (array, not single value)
- `severity`
- `raw_data` (JSON)
- `event_fingerprint` (dedupe helper)

### 2) Server (Rust + Axum)
RabbitMQ:
- Add queue binding for `monitoring.security` and dedicated handler.

Database:
- Add `security_events` table (JSONB evidence + indexed columns).

REST API:
- `GET /api/security`
  - filters: `device_id`, `from`, `to`, `severity`, `mitre_technique`, `query_name`, `limit`
- `GET /api/security/:device_id`
  - same filters except explicit device path
- `GET /api/security/summary`
  - grouped counts by `severity` + `mitre_technique`

### 3) Dashboard (React + TypeScript)
Add page/tab: `Seguridad`
- Table columns:
  - date/time
  - device
  - query name
  - MITRE ATT&CK technique (link to attack.mitre.org)
  - severity badge
  - detail (expand/modal)
- Filters:
  - device
  - date/range
  - severity
  - MITRE technique
- Summary cards:
  - total alerts today
  - critical alerts
  - affected devices
  - top technique
- Export CSV using existing pattern.

---

## Additional Requested Features (Included in Roadmap)

### A) Search events by date from dashboard
Implementation notes:
- Reuse existing `from/to` filter style already used in activity APIs.
- Normalize timezone handling in backend (UTC storage, client offset support).

### B) USB copy alerts
Detection strategy:
- Correlate USB mount/connect events with file write activity in removable volumes.
- Prefer staged rollout:
  1. Rule-based heuristic from existing USB + process/file telemetry.
  2. Optional osquery file_events where available and tuned by platform.

Event taxonomy proposal:
- `USB_COPY_SUSPECTED` (medium)
- `USB_BULK_COPY` (high)
- `USB_COPY_SENSITIVE_PATH` (critical)

### C) Search active users by MAC
Important note:
- Current model stores `device_id` and `mac_address`, but no fully normalized user presence model.

Required additions:
- Define `active_user_sessions` source (OS username/session data from agent).
- Add API query by MAC + time window:
  - `GET /api/users/active?mac_address=...&from=...&to=...`
- In dashboard, add security filter panel field `MAC` and user/session result panel.

---

## Data Model Draft

### Table: `security_events`
Columns:
- `id BIGSERIAL PRIMARY KEY`
- `timestamp TIMESTAMPTZ NOT NULL`
- `device_id UUID NOT NULL`
- `query_name TEXT NOT NULL`
- `query_pack TEXT`
- `mitre_techniques JSONB NOT NULL`
- `severity VARCHAR(20) NOT NULL`
- `raw_data JSONB NOT NULL`
- `event_fingerprint VARCHAR(128)`
- `created_at TIMESTAMPTZ DEFAULT NOW()`

Indexes:
- `(device_id, timestamp DESC)`
- `(severity)`
- `GIN (mitre_techniques)`
- optional expression index for common technique search

---

## Delivery Phases

### Phase 0 - Foundation (must-do)
- Align migration strategy (single source of truth for schema).
- Define event contract for security ingestion.
- Define ATT&CK normalization table/mapping policy.

### Phase 1 - Ingestion MVP
- Agent `osquery_runner` with 2-3 high-value packs.
- Server queue consumer for `monitoring.security`.
- Persist in `security_events`.

### Phase 2 - API + Dashboard MVP
- Implement `/api/security` + `/api/security/:device_id` + `/api/security/summary`.
- Build Seguridad tab with filters + summary + MITRE links.
- Add CSV export for security view.

### Phase 3 - Extra capabilities
- Date-first search UX improvements.
- USB copy alert rules and tuning.
- Active users by MAC (requires user-session telemetry).

### Phase 4 - Hardening
- Performance tuning, dedupe, rate controls.
- False-positive reduction and severity recalibration.
- Validation with pilot endpoints.

---

## TODO to Start (Week 1)

1. Define and freeze security event JSON contract (`monitoring.security`).
2. Select initial osquery packs (max 3) for MVP and expected intervals.
3. Prepare MITRE mapping policy (single vs multiple techniques per event).
4. Create SQL migration draft for `security_events` and indexes.
5. Add server queue binding + no-op handler skeleton for `monitoring.security`.
6. Create API request/response contracts for:
   - `GET /api/security`
   - `GET /api/security/:device_id`
   - `GET /api/security/summary`
7. Define dashboard Seguridad page wireframe and filter behavior.
8. Define CSV export schema for security events.
9. Write acceptance criteria for USB copy alerts (what counts as alert).
10. Define telemetry source for active users by MAC and privacy constraints.

---

## Definition of Done (MVP)
- Security events flow end-to-end: Agent -> RabbitMQ -> Server -> DB -> Dashboard.
- Events searchable by date/device/severity/MITRE.
- Summary endpoint returns grouped counts correctly.
- Dashboard exports filtered security results to CSV.
- Basic operational docs for tuning query intervals and false positives.
