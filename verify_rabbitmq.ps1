# RabbitMQ Queue Verification Script
# This script checks if RabbitMQ is running and verifies queue creation

param(
    [string]$RabbitMQHost = "localhost",
    [int]$RabbitMQPort = 15672,
    [string]$RabbitMQUser = "guest",
    [string]$RabbitMQPass = "guest"
)

Write-Host "`n╔═══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      RabbitMQ Queue Verification Script              ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Step 1: Check if RabbitMQ is reachable
Write-Host "1️⃣  Checking RabbitMQ connectivity..." -ForegroundColor Yellow

$uri = "http://${RabbitMQHost}:${RabbitMQPort}/api/overview"
$auth = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes("${RabbitMQUser}:${RabbitMQPass}"))

try {
    $response = Invoke-WebRequest -Uri $uri -Headers @{"Authorization" = "Basic $auth"} -ErrorAction Stop
    Write-Host "✅ RabbitMQ is accessible at http://${RabbitMQHost}:${RabbitMQPort}" -ForegroundColor Green
} catch {
    Write-Host "❌ Cannot connect to RabbitMQ at http://${RabbitMQHost}:${RabbitMQPort}" -ForegroundColor Red
    Write-Host "   Error: $_" -ForegroundColor Red
    Write-Host "`n💡 Make sure RabbitMQ is running: docker-compose up -d rabbitmq" -ForegroundColor Yellow
    exit 1
}

# Step 2: Check if exchange exists
Write-Host "`n2️⃣  Checking for 'monitoring' exchange..." -ForegroundColor Yellow

$exchangeUri = "http://${RabbitMQHost}:${RabbitMQPort}/api/exchanges/%2F/monitoring"

try {
    $exchangeResponse = Invoke-WebRequest -Uri $exchangeUri -Headers @{"Authorization" = "Basic $auth"} -ErrorAction Stop
    $exchange = $exchangeResponse.Content | ConvertFrom-Json
    Write-Host "✅ Exchange 'monitoring' exists" -ForegroundColor Green
    Write-Host "   Type: $($exchange.type)" -ForegroundColor Gray
    Write-Host "   Durable: $($exchange.durable)" -ForegroundColor Gray
} catch {
    Write-Host "⚠️  Exchange 'monitoring' not found" -ForegroundColor Yellow
    Write-Host "   The server will create it automatically when started" -ForegroundColor Gray
}

# Step 3: Check queues
Write-Host "`n3️⃣  Checking for queues..." -ForegroundColor Yellow

$queuesUri = "http://${RabbitMQHost}:${RabbitMQPort}/api/queues/%2F"

try {
    $queuesResponse = Invoke-WebRequest -Uri $queuesUri -Headers @{"Authorization" = "Basic $auth"} -ErrorAction Stop
    $queues = $queuesResponse.Content | ConvertFrom-Json
    
    $expectedQueues = @("activity_logs", "inventory_logs", "security_alerts")
    
    if ($queues.Count -eq 0) {
        Write-Host "⚠️  No queues found" -ForegroundColor Yellow
        Write-Host "   Expected 3 queues:" -ForegroundColor Gray
        foreach ($q in $expectedQueues) {
            Write-Host "   - $q" -ForegroundColor Gray
        }
        Write-Host "`n💡 Start the server to create queues: cd server && cargo run" -ForegroundColor Yellow
    } else {
        Write-Host "✅ Found $($queues.Count) queue(s):" -ForegroundColor Green
        
        foreach ($queue in $queues) {
            $name = $queue.name
            $messages = $queue.messages
            $durable = $queue.durable
            
            $queueStatus = if ($expectedQueues -contains $name) { "✅" } else { "⚠️ " }
            
            Write-Host "   $queueStatus $name" -ForegroundColor $(if ($expectedQueues -contains $name) { "Green" } else { "Yellow" })
            Write-Host "      Messages: $messages | Durable: $durable" -ForegroundColor Gray
        }
        
        # Check if all expected queues exist
        $missingQueues = $expectedQueues | Where-Object { $queues.name -notcontains $_ }
        if ($missingQueues) {
            Write-Host "`n⚠️  Missing queues:" -ForegroundColor Yellow
            foreach ($missing in $missingQueues) {
                Write-Host "   - $missing" -ForegroundColor Yellow
            }
        } else {
            Write-Host "`n✅ All expected queues exist!" -ForegroundColor Green
        }
    }
} catch {
    Write-Host "❌ Error fetching queues: $_" -ForegroundColor Red
}

# Step 4: Check bindings
Write-Host "`n4️⃣  Checking exchange bindings..." -ForegroundColor Yellow

$bindingsUri = "http://${RabbitMQHost}:${RabbitMQPort}/api/exchanges/%2F/monitoring/bindings/source"

try {
    $bindingsResponse = Invoke-WebRequest -Uri $bindingsUri -Headers @{"Authorization" = "Basic $auth"} -ErrorAction Stop
    $bindings = $bindingsResponse.Content | ConvertFrom-Json
    
    if ($bindings.Count -eq 0) {
        Write-Host "⚠️  No bindings found" -ForegroundColor Yellow
    } else {
        Write-Host "✅ Found $($bindings.Count) binding(s):" -ForegroundColor Green
        foreach ($binding in $bindings) {
            Write-Host "   - $($binding.destination) (routing key: $($binding.routing_key))" -ForegroundColor Gray
        }
    }
} catch {
    # Silently ignore if exchange doesn't exist yet
}

# Step 5: Dashboard info
Write-Host "`n5️⃣  RabbitMQ Management Dashboard" -ForegroundColor Yellow
Write-Host "   URL: http://${RabbitMQHost}:${RabbitMQPort}/" -ForegroundColor Cyan
Write-Host "   User: ${RabbitMQUser}" -ForegroundColor Cyan
Write-Host "   Pass: ${RabbitMQPass}" -ForegroundColor Cyan

# Summary
Write-Host "`n╔═══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                    SUMMARY                            ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host @"

📋 NEXT STEPS:

1. If no queues exist:
   cd server
   RUST_LOG=info cargo run
   
2. Monitor server logs for these messages:
   ✅ RabbitMQ Queues initialized
   ✅ Exchange 'monitoring' declared successfully
   ✅ Queue 'activity_logs' created
   ✅ Queue 'inventory_logs' created
   ✅ Queue 'security_alerts' created

3. Once queues exist, start the agent:
   cd agent
   RUST_LOG=info cargo run

4. Monitor for message flow:
   - Agent publishes: 📤 Publishing event: ...
   - Server receives: ✅ Activity event received: ...
   - Queue messages increase in dashboard

5. For more details, see:
   RABBITMQ_QUEUE_VERIFICATION.md

" -ForegroundColor Green
