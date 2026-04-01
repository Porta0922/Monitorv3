# RabbitMQ Queue Creation - Quick Check

## TL;DR - The Fix

**Problem**: RabbitMQ panel showed no queues, so agent couldn't send data.

**Solution**: Added explicit queue creation with detailed logging.

**Result**: ✅ 3 queues now created automatically when server starts.

---

## 5-Minute Verification

### 1. Start Services
```powershell
docker-compose up -d rabbitmq postgres
```

### 2. Start Server & Watch Logs
```powershell
cd server
RUST_LOG=info cargo run
```

**Look for these lines in output:**
```
✅ RabbitMQ Queues initialized
✅ Queue 'activity_logs' created
✅ Queue 'inventory_logs' created
✅ Queue 'security_alerts' created
```

### 3. Open RabbitMQ Dashboard
http://localhost:15672 (guest / guest)

**Check Queues tab - should show 3 queues:**
```
✅ activity_logs     (Durable: YES, Messages: 0)
✅ inventory_logs    (Durable: YES, Messages: 0)
✅ security_alerts   (Durable: YES, Messages: 0)
```

### 4. Start Agent in New Terminal
```powershell
cd agent
RUST_LOG=info cargo run
```

### 5. Watch Message Count Increase
Refresh http://localhost:15672 every 10 seconds

Queue messages should increase:
```
activity_logs:    N messages ➜ N+1 messages ➜ N+2 messages ...
inventory_logs:   M messages ➜ M+1 messages ➜ M+2 messages ...
```

---

## What Changed

### Server Side
| File | Change | Purpose |
|------|--------|---------|
| `server/src/rabbitmq_consumer.rs` | Added logging at each step | **Visibility** - see exactly what happens |
| Same file | Explicit `queue_declare` options | **Durability** - queues survive restart |
| Same file | Added error messages with `❌` | **Debugging** - clear error visibility |

### Agent Side
| File | Change | Purpose |
|------|--------|---------|
| `agent/src/rabbitmq_publisher.rs` | Added publishing logs | **Verification** - confirm events sent |

---

## Queue Configuration

| Queue | Routing Key | Durable | Purpose |
|-------|------------|---------|---------|
| `activity_logs` | `monitoring.activity` | ✅ | App usage tracking |
| `inventory_logs` | `monitoring.inventory` | ✅ | Software inventory |
| `security_alerts` | `monitoring.security` | ✅ | Security events |

---

## Troubleshooting

| Problem | Check | Fix |
|---------|-------|-----|
| ❌ Queues don't appear | Server logs for `❌ Failed to declare queue` | Check RabbitMQ is running |
| ❌ Dashboard unreachable | `docker ps \| grep rabbitmq` | Restart: `docker-compose up -d rabbitmq` |
| ❌ Agent can't connect | Agent logs for connection errors | Verify `RABBITMQ_URL=amqp://guest:guest@localhost:5672/` |
| ❌ No message increase | Check agent is publishing | Look for `📤 Publishing event` logs in agent |

---

## Log Examples

### ✅ Successful Server Output (Key Lines)
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
✅ RabbitMQ Queues initialized  ⬅️ CRITICAL LINE
📡 RabbitMQ consumer started, listening to monitoring.* events
```

### ✅ Successful Agent Output (Key Lines)
```
🔌 Agent connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
✅ Agent connected to RabbitMQ
📢 Agent declaring 'monitoring' exchange (Topic, Durable)
✅ Agent 'monitoring' exchange declared successfully
📤 Publishing event: activity (routing_key: monitoring.activity)
✅ Event published successfully: monitoring.activity (285 bytes)
```

---

## Success Indicators

✅ **Your system is working when:**

1. ✅ Server shows: `✅ RabbitMQ Queues initialized`
2. ✅ Dashboard shows 3 queues in "Queues" tab
3. ✅ Each queue has "Durable: Yes"
4. ✅ Agent starts without connection errors
5. ✅ Queue message count increases over time
6. ✅ Server logs show: `Activity event received: {...}`

---

## Files Modified

**Production Code:**
- `server/src/rabbitmq_consumer.rs` - Queue initialization with logging
- `agent/src/rabbitmq_publisher.rs` - Event publishing with logging

**Documentation:**
- `RABBITMQ_QUEUE_VERIFICATION.md` - Complete 12KB verification guide
- `verify_rabbitmq.ps1` - Automated PowerShell verification script
- `RABBITMQ_QUICK_CHECK.md` - This file (quick reference)

---

## Next Steps

1. **✅ Verify queues exist** (follow 5-minute check above)
2. **🔧 Connect to PostgreSQL** - Implement database storage
3. **📊 Process events** - Parse JSON and insert into tables
4. **📱 Dashboard display** - Query database and show results
5. **🔔 Alerts** - Implement real-time notifications

---

## Quick Links

- **RabbitMQ Dashboard**: http://localhost:15672 (guest/guest)
- **Full Verification**: See `RABBITMQ_QUEUE_VERIFICATION.md`
- **Automated Script**: Run `.\verify_rabbitmq.ps1`
- **Server API**: http://localhost:3000
- **Dashboard**: http://localhost:5173 (coming soon)

---

## Support

If queues still don't appear:

1. Run the verification script:
   ```powershell
   .\verify_rabbitmq.ps1
   ```

2. Check full logs:
   ```powershell
   # Server
   RUST_LOG=debug cargo run
   
   # Agent
   RUST_LOG=debug cargo run
   ```

3. See `RABBITMQ_QUEUE_VERIFICATION.md` for advanced troubleshooting
