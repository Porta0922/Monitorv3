# 🔍 Dashboard No Showing Data - Diagnostic Checklist

## Quick Diagnostic Test

Run these checks **IN ORDER** to find what's not working:

---

## ✅ Step 1: Verify Docker Services

```powershell
# Check if RabbitMQ is running
docker ps | grep rabbitmq

# Expected output:
# CONTAINER ID   IMAGE          STATUS
# abc123...      rabbitmq:3.x   Up XX seconds
```

**If RabbitMQ is NOT running:**
```powershell
docker-compose up -d rabbitmq postgres
```

---

## ✅ Step 2: Check RabbitMQ Panel

Open: **http://localhost:15672**
- Login: `guest` / `guest`
- Go to **"Queues"** tab

**Expected to see:**
```
✅ inventory_queue (Messages: N where N > 0)
✅ activity_queue (Messages: N where N > 0)
```

**If no queues appear:**
- Server hasn't started yet
- Or server failed to create queues
- Check server logs for errors

**If queues exist but have 0 messages:**
- Agent isn't publishing events
- Check agent logs for errors

---

## ✅ Step 3: Check Server Logs

In the terminal where server is running, look for:

```
✅ RabbitMQ Queues initialized
✅ Activity event received: {...}
✅ Inventory event received: {...}
```

**If NOT seeing these:**
- Agent is not sending data
- Check agent is actually running
- Check agent logs

---

## ✅ Step 4: Check Agent Logs

In the terminal where agent is running, look for:

```
📤 Publishing event: activity (routing_key: monitoring.activity)
✅ Event published successfully: monitoring.activity (XXX bytes)
📤 Publishing event: inventory (routing_key: monitoring.inventory)
✅ Event published successfully: monitoring.inventory (XXX bytes)
```

**If NOT seeing these:**
- Agent failed to connect to RabbitMQ
- Check agent error logs
- Verify `RABBITMQ_URL` in agent code is: `amqp://guest:guest@localhost:5672/`

---

## ✅ Step 5: Check Dashboard Network Call

Open browser **Developer Tools** (F12):
1. Go to **"Network"** tab
2. Refresh dashboard (F5)
3. Look for these requests:

```
GET  /api/devices          → Should return array of devices
GET  /api/activity         → Should return activity logs
GET  /api/inventory        → Should return inventory
```

**If requests show RED (failed):**
- API endpoint not responding
- CORS error
- Server not running on port 3000

**If requests are GREEN but no data:**
- Database query is returning empty array
- Data not being stored in PostgreSQL

---

## 🎯 Most Likely Problems & Fixes

### Problem 1: RabbitMQ Queues Show 0 Messages
**Cause:** Agent not publishing
**Fix:**
```powershell
# Check agent is running
# Check agent logs for "Publishing event"
# If no logs, agent crashed
# Check RUST_LOG=info cargo run output
```

### Problem 2: Queues Have Messages But Dashboard Shows Nothing
**Cause:** Server receiving events but not storing in DB
**Fix:**
- Check server logs for "Activity event received"
- The handlers are TODO stubs - they don't save to DB!
- Need to implement database storage

### Problem 3: Dashboard Shows "Network Error"
**Cause:** Server not running or API endpoint failed
**Fix:**
```powershell
# Verify server is running
curl http://localhost:3000/api/health

# Check CORS is working
curl -i -X OPTIONS http://localhost:3000/api/auth/login \
  -H "Origin: http://localhost:5173" \
  -H "Access-Control-Request-Method: POST"
```

### Problem 4: Dashboard Shows Data But It's Wrong/Incomplete
**Cause:** API endpoint exists but database query is wrong
**Fix:**
- Check database schema exists
- Verify migrations ran
- Check server logs for SQL errors

---

## 🔧 The Root Cause (Most Likely)

The event handlers in `server/src/rabbitmq_consumer.rs` are **TODO stubs**:

```rust
async fn handle_activity_event(event: &Value) {
    info!("✅ Activity event received: {:?}", event);
    // TODO: Parse event and insert into activity_logs table
    // TODO: Validate device_id exists
    // TODO: Extract app_name, window_title, duration_seconds
    // TODO: INSERT into activity_logs hypertable
}
```

**What this means:**
- ✅ Events ARE being received from RabbitMQ
- ❌ Events are NOT being saved to PostgreSQL
- ❌ API queries return empty because database is empty
- ❌ Dashboard shows nothing because API returns nothing

---

## 📊 Data Flow Diagram

```
Agent publishes
    ↓
RabbitMQ receives (✅ Working - you see messages in queue)
    ↓
Server consumes (✅ Working - logs show "event received")
    ↓
Database stores (❌ NOT WORKING - handlers are TODO stubs)
    ↓
API returns data (❌ NOT WORKING - no data in database)
    ↓
Dashboard displays (❌ NOT WORKING - API returns empty)
```

---

## ✅ Quick Verification Script

Run this PowerShell script to test the entire chain:

```powershell
# 1. Check Docker
"[1] Checking Docker..."
$rabbit = docker ps | Select-String rabbitmq
if ($rabbit) { "✅ RabbitMQ running" } else { "❌ RabbitMQ NOT running" }

# 2. Check RabbitMQ API
"[2] Checking RabbitMQ API..."
$queues = curl -s -u guest:guest http://localhost:15672/api/queues/%2F | jq '.[] | .name'
"Queues: $queues"

# 3. Check Server
"[3] Checking Server API..."
$server = curl -s http://localhost:3000/api/devices
"Devices response: $server"

# 4. Check Dashboard
"[4] Opening Dashboard..."
start http://localhost:5173
```

---

## 🚀 Solution: Implement Database Storage

To make data appear in dashboard, you need to:

1. **Implement `handle_activity_event()`**
   - Parse JSON event
   - Extract device_id, app_name, window_title, duration
   - INSERT into PostgreSQL activity_logs table

2. **Implement `handle_inventory_event()`**
   - Parse JSON event
   - Extract device_id, software list
   - INSERT into PostgreSQL app_inventory table

3. **Update API endpoints**
   - GET /api/devices → query devices table
   - GET /api/activity/:device_id → query activity_logs
   - GET /api/inventory/:device_id → query app_inventory

4. **Update Dashboard**
   - Fetch from API endpoints
   - Display results in tables/charts

---

## 📝 Next Actions

**Option A: Quick Check (2 min)**
```
1. Open http://localhost:15672
2. Check if activity_queue and inventory_queue have N > 0 messages
3. Check server logs for "Activity event received"
4. If yes → problem is database storage (handlers are TODOs)
```

**Option B: Implement Database Storage (30 min)**
```
1. Create PostgreSQL tables for devices, activity_logs, app_inventory
2. Implement handle_activity_event() to INSERT
3. Implement handle_inventory_event() to INSERT
4. Test: Agent publishes → Data appears in dashboard
```

**Option C: Debug Step by Step**
```
1. Verify Agent → RabbitMQ (check queue messages)
2. Verify Server → RabbitMQ (check logs)
3. Verify Database → Server (check if INSERT works)
4. Verify API → Dashboard (check if SELECT works)
```

---

## 💡 If All Else Fails

Kill everything and restart cleanly:

```powershell
# 1. Stop all services
docker-compose down
Stop-Process -Name "activity-monitor-server" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "activity-monitor-agent" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue

# 2. Clean up
rm -r server/target
rm -r agent/target
rm -r dashboard/node_modules

# 3. Rebuild and restart
docker-compose up -d
cd server && cargo run --release
# In another terminal:
cd agent && cargo run --release
# In another terminal:
cd dashboard && npm run dev
```

---

## 📞 Support Info

If still stuck, tell me:
1. What's running: RabbitMQ? ✅/❌ | Server? ✅/❌ | Agent? ✅/❌ | Dashboard? ✅/❌
2. What you see in RabbitMQ dashboard (queue count)
3. What you see in server logs (events received?)
4. What you see in dashboard (error? empty table? error in console?)

Then I can pinpoint exactly what's broken.
