# Queue Naming Update - inventory_queue & activity_queue

## Summary

Changed RabbitMQ queue names to match standard naming convention and allow independent processing of inventory and activity events.

## What Changed

### Before
```
Queue 1: activity_logs (routing key: monitoring.activity)
Queue 2: inventory_logs (routing key: monitoring.inventory)
Queue 3: security_alerts (routing key: monitoring.security)
```

### After
```
Queue 1: inventory_queue (routing key: monitoring.inventory)
Queue 2: activity_queue (routing key: monitoring.activity)
```

## Key Features

✅ **Ignores .env queue configuration** - Server always creates standard queues regardless of what .env defines

✅ **Durable queues** - Both queues use `durable: true`, so they persist in RabbitMQ panel even without messages

✅ **Standard naming** - Simpler queue names:
- `inventory_queue` (not `inventory_logs`)
- `activity_queue` (not `activity_logs`)

✅ **Separate processing** - Two dedicated queues for independent handling:
- `inventory_queue` → handle_inventory_event()
- `activity_queue` → handle_activity_event()

✅ **Clean logging** - Each event handler logs with `✅` prefix for visibility

## Files Modified

### server/src/rabbitmq_consumer.rs

**Changes:**
1. Queue creation (lines 53-65):
   - Changed from 3 queues to 2 queues
   - Updated names: `activity_logs` → `activity_queue`
   - Updated names: `inventory_logs` → `inventory_queue`
   - Removed: `security_alerts` queue (no longer needed)
   - Added comment: "ignoring .env queue configuration"

2. Event matching (lines 146-154):
   - Updated match statements to use new queue names
   - Only handle `activity_queue` and `inventory_queue`
   - Removed `security_alerts` case

3. Event handlers (lines 171-187):
   - Removed `handle_security_event()` method
   - Updated logging with `✅` prefix
   - Kept TODO comments for database integration

## Log Output

### Server Startup
```
🔌 Connecting to RabbitMQ at: amqp://guest:guest@localhost:5672/
✅ Connected to RabbitMQ
📢 Declaring 'monitoring' exchange (Topic, Durable)
✅ Exchange 'monitoring' declared successfully
🏗️  Creating standard queues (ignoring .env queue configuration)...
  📋 Creating queue 'inventory_queue' (Durable: true)
  ✅ Queue 'inventory_queue' created
  🔗 Binding 'inventory_queue' to exchange 'monitoring' with routing key 'monitoring.inventory'
  ✅ Queue 'inventory_queue' bound successfully
  🎧 Consumer started for queue 'inventory_queue'
  📋 Creating queue 'activity_queue' (Durable: true)
  ✅ Queue 'activity_queue' created
  🔗 Binding 'activity_queue' to exchange 'monitoring' with routing key 'monitoring.activity'
  ✅ Queue 'activity_queue' bound successfully
  🎧 Consumer started for queue 'activity_queue'
✅ RabbitMQ Queues initialized
📡 RabbitMQ consumer started, listening to monitoring.* events
```

### When Receiving Events
```
✅ Inventory event received: {...}
✅ Activity event received: {...}
```

## Verification

### In RabbitMQ Dashboard (http://localhost:15672)

Navigate to "Queues" tab and verify:

```
✅ inventory_queue
   - Durable: Yes
   - Messages: 0+ (will increase as events arrive)

✅ activity_queue
   - Durable: Yes
   - Messages: 0+ (will increase as events arrive)
```

### From Command Line

```powershell
# Check queues exist
curl -u guest:guest http://localhost:15672/api/queues/%2F | jq '.[] | .name'

# Expected output:
# "inventory_queue"
# "activity_queue"
```

## Why This Change?

1. **Simplicity** - Fewer, clearer names
2. **Focus** - Two distinct data streams instead of three
3. **Scalability** - Easy to add more queues later if needed
4. **Standards** - Matches naming conventions for queue-based systems
5. **.env Independence** - Server creates standard queues regardless of configuration

## Next Steps

1. Verify queues appear in RabbitMQ dashboard
2. Start agent and confirm events are routed correctly
3. Implement database storage in event handlers
4. Add real-time dashboard updates

## Backward Compatibility

⚠️ **Breaking Change**: If agent still publishes to old routing keys:
- Old: `monitoring.activity` → goes to `activity_queue` ✅ (still works)
- Old: `monitoring.inventory` → goes to `inventory_queue` ✅ (still works)

Agent should continue to work without changes since routing keys remain the same.
