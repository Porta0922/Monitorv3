# WebSocket Real-Time Synchronization

**Status**: Architecture Designed | Implementation Ready | Not Yet Integrated

This document describes the WebSocket real-time synchronization system for ActivityMonitor Enterprise v3 dashboard.

## Overview

The WebSocket layer enables **real-time, bi-directional communication** between the server and dashboard clients, eliminating the need for polling and providing instant updates.

## Architecture

```
Dashboard (React Client)
  ↓ (WebSocket Connection)
Server (Rust + Tokio)
  ├─ /ws/devices?token=JWT (WebSocket endpoint)
  ├─ Manages connections
  ├─ Broadcasts updates
  └─ Handles subscriptions

Broadcasting Flow:
RabbitMQ Event → Server Handler → WsSubscriber → All Connected Clients
```

## Message Types

### Client → Server

```json
{
  "type": "subscribe",
  "device_ids": ["device-1", "device-2"]
}
```

```json
{
  "type": "ping"
}
```

### Server → Client

**Device Status Update**
```json
{
  "type": "device_status",
  "data": {
    "device_id": "abc-123",
    "online": true,
    "last_seen": "2025-01-15T14:30:00Z"
  }
}
```

**Activity Log Entry**
```json
{
  "type": "activity_log",
  "data": {
    "device_id": "abc-123",
    "app_name": "Visual Studio Code",
    "window_title": "main.rs",
    "timestamp": "2025-01-15T14:30:15Z"
  }
}
```

**USB Event**
```json
{
  "type": "usb_event",
  "data": {
    "device_id": "abc-123",
    "action": "IN",
    "hardware_id": "0x1234:0x5678",
    "device_name": "SanDisk Ultra",
    "timestamp": "2025-01-15T14:30:20Z"
  }
}
```

**Security Alert**
```json
{
  "type": "security_alert",
  "data": {
    "device_id": "abc-123",
    "alert_type": "HASH_MISMATCH",
    "severity": "HIGH",
    "app_name": "notepad.exe",
    "description": "Executable hash changed",
    "timestamp": "2025-01-15T14:30:25Z"
  }
}
```

## Implementation Steps

### Step 1: Server Side (Rust)

Add to `Cargo.toml`:
```toml
tokio-tungstenite = "0.20"
futures = "0.3"
```

Add to `server/src/main.rs`:
```rust
mod ws;

use ws::WsSubscriber;
use axum::extract::ws::WebSocketUpgrade;

// Create subscriber instance
let ws_subscriber = WsSubscriber::new();

// Add WebSocket route
.route("/ws", get(ws_handler))

// Integrate with RabbitMQ consumer
// When event received: ws_subscriber.broadcast(device_id, message).await
```

### Step 2: Dashboard Side (React)

Create `dashboard/src/hooks/useWebSocket.ts`:
```typescript
import { useEffect, useState } from 'react';

export function useWebSocket(token: string) {
  const [ws, setWs] = useState<WebSocket | null>(null);
  const [messages, setMessages] = useState([]);

  useEffect(() => {
    const socket = new WebSocket(
      `ws://localhost:3000/ws?token=${token}`
    );

    socket.onopen = () => console.log('WebSocket connected');
    socket.onmessage = (e) => {
      const msg = JSON.parse(e.data);
      setMessages(prev => [...prev, msg]);
    };
    socket.onclose = () => console.log('WebSocket disconnected');

    setWs(socket);
    return () => socket.close();
  }, [token]);

  return { ws, messages };
}
```

### Step 3: Update Dashboard Components

Replace polling with WebSocket updates:

**Before** (polling):
```typescript
useEffect(() => {
  const interval = setInterval(async () => {
    const logs = await apiClient.getActivityLogs();
    setLogs(logs);
  }, 5000); // Poll every 5 seconds
}, []);
```

**After** (real-time):
```typescript
const { messages } = useWebSocket(token);

useEffect(() => {
  messages.forEach(msg => {
    if (msg.type === 'activity_log') {
      setLogs(prev => [msg.data, ...prev]);
    }
  });
}, [messages]);
```

### Step 4: Integration with RabbitMQ

In `rabbitmq_consumer.rs`:

```rust
// When activity event received from RabbitMQ
for event in events {
    // Store in database
    db.insert_activity_log(&event).await;
    
    // Broadcast to WebSocket subscribers
    ws_subscriber.broadcast(
        &event.device_id,
        WsMessage::ActivityLog {
            device_id: event.device_id,
            app_name: event.app_name,
            window_title: event.window_title,
            timestamp: event.timestamp,
        }
    ).await;
}
```

## Benefits

| Feature | Before (Polling) | After (WebSocket) |
|---------|------------------|-------------------|
| **Latency** | 5-10 seconds | <100ms |
| **Bandwidth** | High (constant polling) | Low (event-driven) |
| **Server Load** | Higher (many queries) | Lower (events only) |
| **User Experience** | Delayed updates | Real-time |
| **Battery Impact** | Higher (mobile) | Lower (event-driven) |
| **Scalability** | Limited | Better (persistent connections) |

## Connection Limits

With WebSocket persistent connections:
- **Server Memory**: ~1 KB per connection
- **Max Connections**: 10,000+ per server (with optimization)
- **Bandwidth per Connection**: <1 KB/min (event-driven)

## Security Considerations

### Authentication
```rust
// WebSocket endpoint requires valid JWT
GET /ws?token=eyJhbGciOiJIUzI1NiIs...

// Server validates token before accepting connection
if !is_valid_jwt(&token) {
    return Err(Rejection::unauthorized());
}
```

### Message Validation
- All messages must be valid JSON
- Device IDs checked against user's permissions
- Max message size: 10 KB (prevent abuse)

### Heartbeat/Ping
- Client sends ping every 30 seconds
- Server responds with pong
- Connection dropped after 2 missed pings
- Prevents zombie connections

## Performance Considerations

### Optimization 1: Selective Broadcasting
Only broadcast to subscribed clients:
```rust
// Don't broadcast to devices nobody is viewing
if subscriber.has_subscribers(device_id).await {
    subscriber.broadcast(device_id, message).await;
}
```

### Optimization 2: Message Compression
```rust
// Compress large payloads
let compressed = zstd::encode_all(&json_bytes)?;
socket.send(Message::Binary(compressed)).await;
```

### Optimization 3: Rate Limiting
```rust
// Don't spam client with events
// Buffer activity logs and send batches every 1 second
// Instead of: 100 activity logs per second
// Send: 1 batch of 100 logs per second
```

## Testing

### Unit Tests (in `server/src/ws.rs`)

```rust
#[tokio::test]
async fn test_websocket_subscriber() {
    let subscriber = WsSubscriber::new();
    let mut rx = subscriber.subscribe("device-1".to_string()).await;
    
    subscriber.broadcast("device-1", WsMessage::Ping).await;
    assert!(rx.recv().await.is_some());
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let subscriber = WsSubscriber::new();
    let mut rx1 = subscriber.subscribe("device-1".to_string()).await;
    let mut rx2 = subscriber.subscribe("device-1".to_string()).await;
    
    subscriber.broadcast("device-1", WsMessage::Ping).await;
    assert!(rx1.recv().await.is_some());
    assert!(rx2.recv().await.is_some());
}
```

### Integration Tests

1. Connect client to WebSocket
2. Send activity event via RabbitMQ
3. Verify client receives message
4. Measure latency (<100ms)

## Deployment Checklist

- [ ] Add WebSocket dependencies to `Cargo.toml`
- [ ] Create `server/src/ws.rs` module
- [ ] Add WebSocket endpoint to `server/src/api.rs`
- [ ] Integrate with RabbitMQ consumer
- [ ] Create React `useWebSocket` hook
- [ ] Update dashboard components to use WebSocket
- [ ] Add heartbeat/ping mechanism
- [ ] Add rate limiting
- [ ] Load test with 100+ concurrent connections
- [ ] Monitor connection stability in production
- [ ] Add logging and metrics

## Fallback Strategy

If WebSocket connection drops:
1. Dashboard automatically falls back to HTTP polling (5-second interval)
2. Shows "Updating..." indicator to user
3. Attempts WebSocket reconnection every 10 seconds
4. Switches back to WebSocket when reconnected

```typescript
const [usePolling, setUsePolling] = useState(false);

if (ws?.readyState !== WebSocket.OPEN) {
  // Fall back to polling
  setUsePolling(true);
} else {
  setUsePolling(false);
}
```

## Monitoring & Metrics

Track WebSocket health:
- Active connections count
- Message latency (p50, p95, p99)
- Connection drop rate
- Error rate
- Bandwidth usage

```rust
// In RabbitMQ consumer
let start = Instant::now();
ws_subscriber.broadcast(&device_id, message).await;
let latency = start.elapsed();

metrics::histogram!("ws.broadcast.latency", latency);
metrics::counter!("ws.broadcasts.total", 1);
```

## Future Enhancements

1. **Connection Clustering**: Multiple server instances share WebSocket load
2. **Redis PubSub**: Broadcast across multiple servers using Redis
3. **Compression**: Message compression for large payloads
4. **Selective Updates**: Only send changed fields (delta updates)
5. **Client-side Caching**: Cache state locally, reduce message size

## Summary

WebSocket real-time synchronization:
- ✅ Eliminates polling overhead
- ✅ Provides instant updates (<100ms)
- ✅ Reduces server load
- ✅ Improves user experience
- ✅ Works with existing architecture
- ✅ Includes fallback to polling

---

**Ready to implement?** See implementation steps above or contact the engineering team for code review.
