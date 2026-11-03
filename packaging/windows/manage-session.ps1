#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('repair','activate','revoke','rotate','reenroll','migrate')][string]$Action)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
    if ((Get-Service -Name ActivityMonitor).Status -ne 'Stopped') { throw 'Stop ActivityMonitor before changing credentials.' }
    $service = Get-CimInstance Win32_Service -Filter "Name='ActivityMonitor'"
    if ($service.PathName -notmatch '^"([^"\r\n]+)"$') { throw 'Expected a quoted registered binary path without arguments.' }
    $binary = [IO.Path]::GetFullPath($Matches[1])
    foreach ($process in Get-CimInstance Win32_Process) {
        if ($process.ExecutablePath -and [IO.Path]::GetFullPath($process.ExecutablePath) -eq $binary) {
            throw 'Stop desktop agents before changing credentials.'
        }
    }
    $basePath = Join-Path $env:ProgramData 'ActivityMonitor'
    $configPath = Join-Path $basePath '.env'
    $markerPath = Join-Path $basePath 'session-enrollment.json'
    $journalPath = Join-Path $basePath 'session-operation.json'
    $enrollmentPath = Join-Path $basePath 'SessionEnrollment'
    $adminSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
    $systemSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
    $utf8 = [Text.UTF8Encoding]::new($false)
    . (Join-Path $PSScriptRoot 'session-common.ps1')
    foreach ($path in @($basePath,$configPath,$enrollmentPath)) { Assert-NoReparse $path; Assert-AdminControlled $path }
    $pending = Test-Path -LiteralPath $journalPath
    if ($pending -and $Action -ne 'repair') { throw 'Pending operation; run repair first.' }
    $recordPath = if ($pending) { $journalPath } else { $markerPath }
    Assert-NoReparse $recordPath; Assert-AdminControlled $recordPath
    $record = [IO.File]::ReadAllText($recordPath) | ConvertFrom-Json
    $userSid = [Security.Principal.SecurityIdentifier]::new($record.sid)
    if ($userSid.Value -in @('S-1-5-18','S-1-5-19','S-1-5-20','S-1-5-32-544')) { throw 'Invalid enrolled principal.' }
    $null = $userSid.Translate([Security.Principal.NTAccount])
    $userPath = Join-Path $enrollmentPath $userSid.Value
    Assert-NoReparse $userPath; Assert-AdminControlled $userPath
    $sessionPath = Join-Path $userPath 'session.env'
    $original = [IO.File]::ReadAllText($configPath)
    if ($pending) {
        if ($record.version -ne 1 -or $record.final_state -notin @('prepared','active','revoked')) { throw 'Unknown operation journal.' }
        $plan = $record
        if ((Get-SessionSetting $plan.service_text 'AGENT_SESSION_RELAY_KEY') -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid journal relay key.' }
        if ($null -ne $plan.session_text) {
            foreach ($name in @('AGENT_SESSION_RELAY_KEY')) {
                if ((Get-SessionSetting $plan.session_text $name) -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid journal session key.' }
            }
        }
    } else {
        Assert-NoReparse $sessionPath; Assert-AdminControlled $sessionPath
        $current = [IO.File]::ReadAllText($sessionPath)
        $relayKey = Get-SessionSetting $current 'AGENT_SESSION_RELAY_KEY'
        $cacheKey = Get-SessionSetting $current 'AGENT_OFFLINE_CACHE_KEY' $false
        if (!$cacheKey) { $cacheKey = $relayKey }
        $serviceKey = Get-SessionSetting $original 'AGENT_SESSION_RELAY_KEY'
        foreach ($value in @($relayKey,$serviceKey)) { if ($value -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid/custom key; explicit migration required.' } }
        $port = [int](Get-SessionSetting $current 'AGENT_SESSION_RELAY_PORT')
        if ($port -lt 1 -or $port -gt 65535) { throw 'Invalid relay port.' }
        if ($record.state -eq 'revoked' -and $Action -in @('activate','rotate')) { throw 'Revoked session requires explicit re-enrollment.' }
        if ($Action -ne 'migrate' -and $record.state -ne 'revoked' -and $serviceKey -ne $relayKey) { throw 'Unjournaled key mismatch; refusing to guess the correct credential.' }
        if ($Action -eq 'reenroll' -and $record.state -ne 'revoked') { throw 'Re-enrollment requires a revoked session.' }
        $enabled = $Action -eq 'activate' -or ($Action -in @('repair','rotate') -and $record.state -eq 'active')
        $newKey = if ($Action -in @('rotate','revoke','reenroll','migrate')) { New-SessionSecret } else { $serviceKey }
        $cacheSettings = [ordered]@{AGENT_SESSION_RELAY_KEY=$newKey}
        if ($null -eq (Get-SessionSetting $current 'AGENT_OFFLINE_CACHE_KEY' $false)) { $cacheSettings.AGENT_OFFLINE_CACHE_KEY=$cacheKey }
        $sessionText = if ($Action -eq 'revoke' -or ($record.state -eq 'revoked' -and $Action -notin @('reenroll','migrate'))) { $null } else { Update-SessionEnvironment $current $cacheSettings }
        $state = if ($Action -eq 'revoke' -or ($record.state -eq 'revoked' -and $Action -notin @('reenroll','migrate'))) { 'revoked' } elseif ($enabled) { 'active' } else { 'prepared' }
        $serviceText = Update-SessionEnvironment $original ([ordered]@{AGENT_SESSION_RELAY_ENABLED=($enabled.ToString().ToLowerInvariant()); AGENT_SESSION_RELAY_KEY=$newKey; AGENT_SESSION_RELAY_PORT=$port})
        $plan = @{version=1; action=$Action; sid=$userSid.Value; port=$port; service_text=$serviceText; session_text=$sessionText; final_state=$state}
        Write-Protected $journalPath ($plan | ConvertTo-Json) $false
    }
    if ($null -ne $plan.session_text) { Write-Protected $sessionPath $plan.session_text $true }
    Write-Protected $configPath $plan.service_text $false
    Write-Protected $markerPath (@{state=$plan.final_state; sid=$userSid.Value; config=$sessionPath; port=$plan.port} | ConvertTo-Json) $false
    Remove-Item -LiteralPath $journalPath
    Write-Output "Session state: $($plan.final_state). Agents remain stopped. Identity and outbox retained."
} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
