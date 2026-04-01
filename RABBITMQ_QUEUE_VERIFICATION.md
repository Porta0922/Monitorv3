# RabbitMQ Queue Initialization - Verification Guide

## Overview
This document explains the RabbitMQ queue initialization fixes and how to verify they work correctly.

## Changes Made

### 1. **Server (Rust) - `server/src/rabbitmq_consumer.rs`**

#### Fixed Issues:
- ✅ Added comprehensive logging for queue initialization
- ✅ Added error handling with descriptive messages
- ✅ Ensured `queue_declare` uses `durable: true`, `exclusive: false`, `auto_delete: false`
- ✅ Added explicit logging: `✅ RabbitMQ Queues initialized`

#### Queue Creation Flow:
```
1. Connect to RabbitMQ → Log: "🔌 Connecting to RabbitMQ..."
2. Create channel → Log: "✅ Connected to RabbitMQ"
3. Declare exchange → Log: "📢 Declaring 'monitoring' exchange..."
4. Create 3 queues:
   - activity_logs (routing key: monitoring.activity)
   - inventory_logs (routing key: monitoring.inventory)
   - security_alerts (routing key: monitoring.security)
5. Bind queues to exchange → Log per queue
6. Start consumers → Log: "✅ RabbitMQ Queues initialized"
```

### 2. **Agent (Rust) - `agent/src/rabbitmq_publisher.rs`**

#### Fixed Issues:
- ✅ Added connection logging: `"🔌 Agent connecting to RabbitMQ..."`
- ✅ Added error handling with clear error messages
- ✅ Updated `publish_event()` with visible logging
- ✅ Exchange name matches server exactly: `"monitoring"`
- ✅ Routing keys match server exactly: `"monitoring.{event_type}"`

#### Publishing Flow:
```
1. Connect to RabbitMQ → Log: "🔌 Agent connecting to RabbitMQ..."
2. Create channel → Log: "✅ Agent connected to RabbitMQ"
3. Declare exchange → Log: "📢 Agent declaring 'monitoring' exchange..."
4. Publish events with exact routing keys:
   - Activity: "monitoring.activity"
   - Inventory: "monitoring.inventory"
   - Security: "monitoring.security"
   - USB: "monitoring.usb"
5. Each publish logs: "📤 Publishing event: {type} (routing_key: monitoring.{type})"
```

## Verification Steps

### Step 1: Start RabbitMQ (Docker)
```bash
# From project root
docker-compose up -d rabbitmq postgres
```

### Step 2: Check RabbitMQ Logs
```bash
docker logs -f rabbitmq
```
Expected output:
```
rabbitmq_1  | [*] Waiting for connection...
```

### Step 3: Start Server
```bash
cd server
RUST_LOG=info cargo run
```

Expected output in server logs:
```
🔌 Connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
✅ Connected to RabbitMQ
📢 Declaring 'monitoring' exchange (Topic, Durable)
✅ Exchange 'monitoring' declared successfully
🏗️  Creating queues...
  📋 Creating queue 'activity_logs' (Durable: true)
  ✅ Queue 'activity_logs' created
  🔗 Binding 'activity_logs' to exchange 'monitoring' with routing key 'monitoring.activity'
  ✅ Queue 'activity_logs' bound successfully
  🎧 Consumer started for queue 'activity_logs'
  📋 Creating queue 'inventory_logs' (Durable: true)
  ✅ Queue 'inventory_logs' created
  🔗 Binding 'inventory_logs' to exchange 'monitoring' with routing key 'monitoring.inventory'
  ✅ Queue 'inventory_logs' bound successfully
  🎧 Consumer started for queue 'inventory_logs'
  📋 Creating queue 'security_alerts' (Durable: true)
  ✅ Queue 'security_alerts' created
  🔗 Binding 'security_alerts' to exchange 'monitoring' with routing key 'monitoring.security'
  ✅ Queue 'security_alerts' bound successfully
  🎧 Consumer started for queue 'security_alerts'
✅ RabbitMQ Queues initialized
📡 RabbitMQ consumer started, listening to monitoring.* events
```

### Step 4: Verify Queues in RabbitMQ Dashboard

**Option A: Using curl**
```bash
# Check queues
curl -u guest:guest http://localhost:15672/api/queues/%2F

# Response should show 3 queues:
# - activity_logs (durable: true, messages: 0)
# - inventory_logs (durable: true, messages: 0)
# - security_alerts (durable: true, messages: 0)
```

**Option B: Using RabbitMQ Management UI**
1. Open: http://localhost:15672
2. Login: guest / guest
3. Navigate to: "Queues" tab
4. Should see **exactly 3 queues**:
   - ✅ activity_logs
   - ✅ inventory_logs
   - ✅ security_alerts

Each queue should show:
```
Name: activity_logs
Durable: Yes
Features: []
Messages: 0
Message rate (in/out): 0
```

### Step 5: Start Agent
```bash
cd agent
RUST_LOG=info cargo run
```

Expected output in agent logs:
```
🔌 Agent connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
✅ Agent connected to RabbitMQ
📢 Agent declaring 'monitoring' exchange (Topic, Durable)
✅ Agent 'monitoring' exchange declared successfully
```

### Step 6: Verify Event Publishing

After agent starts, monitor server logs for incoming events:
```
✅ Activity event received: {...}
✅ Inventory event received: {...}
✅ Security event received: {...}
```

Monitor agent logs for outgoing events:
```
📤 Publishing event: activity (routing_key: monitoring.activity)
✅ Event published successfully: monitoring.activity (XXX bytes)
📤 Publishing event: inventory (routing_key: monitoring.inventory)
✅ Event published successfully: monitoring.inventory (XXX bytes)
```

### Step 7: Check RabbitMQ Dashboard for Message Count

Go to http://localhost:15672 → Queues

Should see non-zero message counts:
```
activity_logs: N messages
inventory_logs: N messages
security_alerts: N messages (if security events are triggered)
```

## Troubleshooting

### ❌ No Queues Appearing in Dashboard

**Issue**: `queue_declare()` might be failing silently.

**Solution**:
1. Check server logs for errors starting with `❌ Failed to declare queue`
2. Ensure RabbitMQ container is running: `docker ps | grep rabbitmq`
3. Test RabbitMQ connection:
   ```bash
   docker exec -it rabbitmq rabbitmq-diagnostics ping
   ```

### ❌ Queues Appear but Messages Not Flowing

**Issue**: Exchange might not be properly bound to queues.

**Solution**:
1. Check server logs for error: `❌ Failed to bind queue`
2. Verify routing keys match exactly:
   - Server creates: `monitoring.activity`, `monitoring.inventory`, `monitoring.security`
   - Agent publishes to: `monitoring.activity`, `monitoring.inventory`, `monitoring.security`, `monitoring.usb`
3. Check RabbitMQ Dashboard → Exchanges → "monitoring" → Bindings
   Should show 3 bindings to the queues

### ❌ Server Crashes During Queue Creation

**Issue**: RabbitMQ might not be fully initialized.

**Solution**:
1. Wait 10 seconds after Docker startup
2. Check RabbitMQ is accepting connections:
   ```bash
   docker logs rabbitmq | grep "accepting AMQP connections"
   ```
3. Restart server

### ❌ Agent Can't Connect to RabbitMQ

**Issue**: RabbitMQ_URL might be wrong or RabbitMQ is down.

**Solution**:
1. Check `.env` file has correct `RABBITMQ_URL`
2. Expected: `amqp://guest:guest@localhost:5672/`
3. Test connection:
   ```bash
   docker exec -it rabbitmq rabbitmqctl status
   ```

## Queue Configuration Summary

| Queue Name | Routing Key | Durable | Auto-Delete | Purpose |
|-----------|------------|---------|-------------|---------|
| activity_logs | monitoring.activity | ✅ Yes | ❌ No | App usage logs |
| inventory_logs | monitoring.inventory | ✅ Yes | ❌ No | Software inventory |
| security_alerts | monitoring.security | ✅ Yes | ❌ No | Security events |

## Testing with Direct Publishing

```bash
# Connect to RabbitMQ container
docker exec -it rabbitmq bash

# Test publish to activity queue
rabbitmq-admin publish exchange=monitoring routing_key=monitoring.activity payload='{"test":"data"}'

# Check queue has message
rabbitmqctl list_queues name messages
```

## Expected Final State

✅ **After successful initialization:**

```
RabbitMQ Dashboard (http://localhost:15672/):
├── Exchanges:
│   └── monitoring (Topic, Durable)
├── Queues:
│   ├── activity_logs (Durable, N messages)
│   ├── inventory_logs (Durable, N messages)
│   └── security_alerts (Durable, N messages)
├── Bindings:
│   ├── activity_logs ← monitoring (key: monitoring.activity)
│   ├── inventory_logs ← monitoring (key: monitoring.inventory)
│   └── security_alerts ← monitoring (key: monitoring.security)
```

## Log Examples

### Successful Server Startup (Full)
```
[2024-04-01T23:45:00Z INFO] ActivityMonitor Server v0.1.0 starting...
[2024-04-01T23:45:00Z INFO] 🔌 Connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
[2024-04-01T23:45:01Z INFO] ✅ Connected to RabbitMQ
[2024-04-01T23:45:01Z INFO] 📢 Declaring 'monitoring' exchange (Topic, Durable)
[2024-04-01T23:45:01Z INFO] ✅ Exchange 'monitoring' declared successfully
[2024-04-01T23:45:01Z INFO] 🏗️  Creating queues...
[2024-04-01T23:45:01Z INFO]   📋 Creating queue 'activity_logs' (Durable: true)
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'activity_logs' created
[2024-04-01T23:45:01Z INFO]   🔗 Binding 'activity_logs' to exchange 'monitoring' with routing key 'monitoring.activity'
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'activity_logs' bound successfully
[2024-04-01T23:45:01Z INFO]   🎧 Consumer started for queue 'activity_logs'
[2024-04-01T23:45:01Z INFO]   📋 Creating queue 'inventory_logs' (Durable: true)
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'inventory_logs' created
[2024-04-01T23:45:01Z INFO]   🔗 Binding 'inventory_logs' to exchange 'monitoring' with routing key 'monitoring.inventory'
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'inventory_logs' bound successfully
[2024-04-01T23:45:01Z INFO]   🎧 Consumer started for queue 'inventory_logs'
[2024-04-01T23:45:01Z INFO]   📋 Creating queue 'security_alerts' (Durable: true)
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'security_alerts' created
[2024-04-01T23:45:01Z INFO]   🔗 Binding 'security_alerts' to exchange 'monitoring' with routing key 'monitoring.security'
[2024-04-01T23:45:01Z INFO]   ✅ Queue 'security_alerts' bound successfully
[2024-04-01T23:45:01Z INFO]   🎧 Consumer started for queue 'security_alerts'
[2024-04-01T23:45:01Z INFO] ✅ RabbitMQ Queues initialized
[2024-04-01T23:45:01Z INFO] 📡 RabbitMQ consumer started, listening to monitoring.* events
[2024-04-01T23:45:01Z INFO] Server listening on http://0.0.0.0:3000
```

### Successful Agent Startup (Key Logs)
```
[2024-04-01T23:45:10Z INFO] 🔌 Agent connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
[2024-04-01T23:45:11Z INFO] ✅ Agent connected to RabbitMQ
[2024-04-01T23:45:11Z INFO] 📢 Agent declaring 'monitoring' exchange (Topic, Durable)
[2024-04-01T23:45:11Z INFO] ✅ Agent 'monitoring' exchange declared successfully
[2024-04-01T23:45:12Z INFO] 📤 Publishing event: activity (routing_key: monitoring.activity)
[2024-04-01T23:45:12Z INFO] ✅ Event published successfully: monitoring.activity (285 bytes)
```

## Key Metrics to Monitor

1. **Queue Message Count**
   - Should increase as agent publishes events
   - Check every 30 seconds in dashboard

2. **Exchange Bindings**
   - Should see exactly 3 bindings
   - All to the "monitoring" exchange

3. **Consumer Status**
   - Consumers should show "Ready" status
   - No errors in server logs

4. **Data Flow**
   - Agent sends events (see 📤 logs in agent)
   - Server receives events (see ✅ logs in server)
   - Queue message count increases

## Success Indicators

✅ **The system is working correctly when:**

1. ✅ Server startup shows all 3 queue creation messages
2. ✅ RabbitMQ dashboard shows 3 queues (activity_logs, inventory_logs, security_alerts)
3. ✅ Each queue is Durable and has 0 auto-delete messages
4. ✅ Exchange "monitoring" has 3 bindings
5. ✅ Agent can connect and publish without errors
6. ✅ Queue message counts increase as agent runs
7. ✅ Server logs show "Activity event received", "Inventory event received" messages

## Next Steps

Once queues are verified:

1. **Connect to Database**: Implement `handle_activity_event()` to insert into PostgreSQL
2. **Add Real Data Processing**: Parse JSON events and store in database
3. **Implement Alerts**: Process security events and create real-time alerts
4. **Dashboard Display**: Fetch data from database and display in React dashboard
