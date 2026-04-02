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
                                          (event handler)      (TODO!)        Display

🔴 MOST LIKELY ISSUE:

The event handlers are TODO stubs - they receive events but don't save to database!

File: server/src/rabbitmq_consumer.rs
Lines: 172-187

The handlers log "Activity event received" but don't execute INSERT statements.

SOLUTION: Implement database storage in the handlers.

📋 NEXT STEPS (Choose One):

1. QUICK FIX (5 min)
   → Implement handle_activity_event() to save to PostgreSQL
   → Implement handle_inventory_event() to save to PostgreSQL
   → Restart server
   → Refresh dashboard

2. VERIFY FIRST (2 min)
   → Open http://localhost:15672
   → Check if queues have messages > 0
   → Check server logs for "Activity event received"
   → If yes → problem is database storage (as expected)

3. FULL SETUP (30 min)
   → Create PostgreSQL schema
   → Implement all handlers
   → Update API endpoints
   → Test end-to-end

" -ForegroundColor Green

Write-Host "`n💾 PostgreSQL Connection Test..." -ForegroundColor Yellow
try {
    $pgResponse = Invoke-WebRequest -Uri "http://localhost:3000/api/devices" -ErrorAction SilentlyContinue
    Write-Host "  ✅ Server can reach database" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Server may not be connected to PostgreSQL" -ForegroundColor Yellow
}

Write-Host "`n📖 For detailed help, see: DIAGNOSTIC_CHECKLIST.md`n" -ForegroundColor Cyan
