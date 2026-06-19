param(
    [string]$ConfigDir,
    [string]$BinDir,
    [string]$AuthToken,
    [string]$OfflineCacheKey,
    [string]$ServerUrl,
    [string]$RabbitMqUrl
)

$ErrorActionPreference = "Stop"

# 1. Write .env file
$envContent = @"
# ActivityMonitor Agent Configuration
AGENT_AUTH_TOKEN=$AuthToken
AGENT_OFFLINE_CACHE_KEY=$OfflineCacheKey
AGENT_SERVER_URL=$ServerUrl
RABBITMQ_URL=$RabbitMqUrl
"@

if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

$envPath = Join-Path $ConfigDir ".env"
Set-Content -Path $envPath -Value $envContent -Encoding ascii
Write-Output "[+] .env escrito en $envPath"

# 2. Create scheduled task for user-session agent
$taskName = "ActivityMonitorUserAgent"
$agentBin = Join-Path $BinDir "activity-monitor-agent.exe"

# Build task XML
$taskXml = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Date>2026-06-19T12:00:00</Date>
    <Author>ActivityMonitor</Author>
    <Description>ActivityMonitor User Agent - captures user session activity (window focus, input, idle)</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <GroupId>S-1-5-32-545</GroupId>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>true</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>4</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>99</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"$agentBin"</Command>
    </Exec>
  </Actions>
</Task>
"@

$taskXmlPath = Join-Path $env:TEMP "ActivityMonitorTask.xml"
Set-Content -Path $taskXmlPath -Value $taskXml -Encoding utf8

# Delete existing task if present
& schtasks /Delete /TN $taskName /F 2>$null

# Create task
$result = & schtasks /Create /XML $taskXmlPath /TN $taskName /F 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Output "[+] Scheduled task '$taskName' created"
} else {
    Write-Warning "[!] Failed to create scheduled task: $result"
    # Fallback: simple ONLOGON task
    & schtasks /Create /SC ONLOGON /TN $taskName /TR "`"$agentBin`"" /F /IT 2>$null
}

Remove-Item -Path $taskXmlPath -Force -ErrorAction SilentlyContinue

Write-Output "[+] Post-install complete"
