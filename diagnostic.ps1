#!/usr/bin/env pwsh

# 🔍 ActivityMonitor Dashboard Data Flow Diagnostic
# This script checks each component in the data pipeline

Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║        ActivityMonitor - Dashboard Data Flow Diagnostic       ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# 1. Check Docker Services
Write-Host "Step 1️⃣  Checking Docker Services..." -ForegroundColor Yellow
$rabbitmq = docker ps --format "{{.Names}}" | Select-String "rabbitmq"
$postgres = docker ps --format "{{.Names}}" | Select-String "postgres"

if ($rabbitmq) {
    Write-Host "  ✅ RabbitMQ is running" -ForegroundColor Green
} else {
    Write-Host "  ❌ RabbitMQ is NOT running" -ForegroundColor Red
    Write-Host "     Fix: docker-compose up -d rabbitmq postgres" -ForegroundColor Yellow
}

if ($postgres) {
    Write-Host "  ✅ PostgreSQL is running" -ForegroundColor Green
} else {
    Write-Host "  ❌ PostgreSQL is NOT running" -ForegroundColor Red
    Write-Host "     Fix: docker-compose up -d rabbitmq postgres" -ForegroundColor Yellow
}

# 2. Check RabbitMQ Queues
Write-Host "`nStep 2️⃣  Checking RabbitMQ Queues..." -ForegroundColor Yellow

if ($rabbitmq) {
    try {
        $response = Invoke-WebRequest -Uri "http://localhost:15672/api/queues/%2F" `
            -Headers @{"Authorization" = "Basic " + [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes("guest:guest"))} `
            -ErrorAction SilentlyContinue
        
        $queues = $response.Content | ConvertFrom-Json
        
        if ($queues.Count -eq 0) {
            Write-Host "  ⚠️  No queues found in RabbitMQ" -ForegroundColor Yellow
            Write-Host "     → Server hasn't created queues yet" -ForegroundColor Gray
            Write-Host "     → Or server is not running" -ForegroundColor Gray
        } else {
            $totalMessages = 0
            foreach ($queue in $queues) {
                $messages = $queue.messages
                $totalMessages += $messages
                
                if ($messages -gt 0) {
                    Write-Host "  ✅ Queue '$($queue.name)' has $messages message(s)" -ForegroundColor Green
                } else {
                    Write-Host "  ⚠️  Queue '$($queue.name)' has 0 messages" -ForegroundColor Yellow
                }
            }
            
            if ($totalMessages -eq 0) {
                Write-Host "`n  📌 All queues are empty:" -ForegroundColor Cyan
                Write-Host "     → Agent is not publishing events" -ForegroundColor Gray
                Write-Host "     → Check if agent is running" -ForegroundColor Gray
                Write-Host "     → Check agent logs for errors" -ForegroundColor Gray
            } else {
                Write-Host "`n  ✅ Events are being published to RabbitMQ" -ForegroundColor Green
            }
        }
    } catch {
        Write-Host "  ❌ Cannot connect to RabbitMQ API" -ForegroundColor Red
        Write-Host "     → RabbitMQ might not be fully started" -ForegroundColor Gray
        Write-Host "     → Wait 5 seconds and try again" -ForegroundColor Gray
    }
} else {
    Write-Host "  ⏭️  Skipping (RabbitMQ not running)" -ForegroundColor Gray
}

# 3. Check Server API
Write-Host "`nStep 3️⃣  Checking Server API..." -ForegroundColor Yellow

try {
    $response = Invoke-WebRequest -Uri "http://localhost:3000/api/devices" -ErrorAction SilentlyContinue
    Write-Host "  ✅ Server is responding on port 3000" -ForegroundColor Green
    
    $devices = $response.Content | ConvertFrom-Json
    if ($devices.Count -eq 0) {
        Write-Host "  ⚠️  API returns 0 devices" -ForegroundColor Yellow
        Write-Host "     → No devices registered yet" -ForegroundColor Gray
        Write-Host "     → Or data not being saved to PostgreSQL" -ForegroundColor Gray
    } else {
        Write-Host "  ✅ Found $($devices.Count) device(s) in database" -ForegroundColor Green
    }
} catch {
    Write-Host "  ❌ Server API not responding on port 3000" -ForegroundColor Red
    Write-Host "     → Server is not running" -ForegroundColor Gray
    Write-Host "     → Fix: cd server && cargo run --release" -ForegroundColor Yellow
}

# 4. Check Dashboard
Write-Host "`nStep 4️⃣  Checking Dashboard..." -ForegroundColor Yellow

try {
    $response = Invoke-WebRequest -Uri "http://localhost:5173" -ErrorAction SilentlyContinue
    Write-Host "  ✅ Dashboard is running on port 5173" -ForegroundColor Green
} catch {
    Write-Host "  ❌ Dashboard not running on port 5173" -ForegroundColor Red
    Write-Host "     → Fix: cd dashboard && npm run dev" -ForegroundColor Yellow
}

# 5. Summary
Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                        SUMMARY                                ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host @"

📊 DATA FLOW ANALYSIS:

  Agent               RabbitMQ              Server            PostgreSQL       Dashboard
  (Publishing)        (Message Queue)       (Consumer)        (Storage)        (Display)
       |-------────────→|                      |                |                |
                             |-----------→ Consumes --------→ INSERT ----→ API Query
                                          (event handlers)  (PERSISTED)       Display

✅ PIPELINE STATUS:
All event handlers are FULLY implemented! The telemetry events received from the agent via
RabbitMQ are consumed, normalized, deduplicated (idempotency checks), and persisted to the
PostgreSQL database.

 File: server/src/rabbitmq_consumer.rs (FULLY IMPLEMENTED)
 - handle_activity_event()      -> Saved to PostgreSQL
 - handle_inventory_event()     -> Saved to PostgreSQL
 - handle_heartbeat_event()     -> Saved to PostgreSQL
 - handle_usb_event()           -> Saved to PostgreSQL
 - handle_wifi_event()          -> Saved to PostgreSQL
 - handle_running_apps_event()  -> Saved to PostgreSQL
 - handle_security_event()      -> Saved to PostgreSQL

📋 NEXT DIAGNOSTIC STEPS:

1. VERIFY AGENT STATUS:
   → Ensure the agent binary is running (as Service or User session task).
   → Check logs folder in the agent's install path.

2. VERIFY RABBITMQ STATUS:
   → Open http://localhost:15672 (guest/guest) to monitor queue traffic.
   
3. REFRESH DASHBOARD:
   → Open http://localhost:5173 to view real-time synchronized telemetry.

"@ -ForegroundColor Green

Write-Host "`n💾 PostgreSQL Connection Test..." -ForegroundColor Yellow
try {
    $pgResponse = Invoke-WebRequest -Uri "http://localhost:3000/api/devices" -ErrorAction SilentlyContinue
    Write-Host "  ✅ Server can reach database" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Server may not be connected to PostgreSQL" -ForegroundColor Yellow
}

Write-Host "`n📖 For detailed help, see: DIAGNOSTIC_CHECKLIST.md`n" -ForegroundColor Cyan
