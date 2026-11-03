#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][string]$SessionUser, [ValidateRange(1,65535)][int]$Port = 47652)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
$service = Get-Service -Name ActivityMonitor -ErrorAction Stop
if ($service.Status -ne 'Stopped') { throw 'Stop ActivityMonitor before enrollment.' }
$userSid = ([System.Security.Principal.NTAccount]::new($SessionUser)).Translate([System.Security.Principal.SecurityIdentifier])
if ($userSid.Value -in @('S-1-5-18','S-1-5-19','S-1-5-20','S-1-5-32-544')) { throw 'Choose an individual desktop account.' }
$basePath = Join-Path $env:ProgramData 'ActivityMonitor'
$configPath = Join-Path $basePath '.env'
$markerPath = Join-Path $basePath 'session-enrollment.json'
$adminSid = [System.Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid = [System.Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$utf8 = [System.Text.UTF8Encoding]::new($false)

. (Join-Path $PSScriptRoot 'session-common.ps1')

Assert-NoReparse $basePath
Assert-NoReparse $configPath
Assert-AdminControlled $basePath
Assert-AdminControlled $configPath
if (Test-Path -LiteralPath $markerPath) { throw 'An enrollment exists; explicit repair/revocation is required.' }
# Restrict only the configuration directory dedicated to enrollment, not existing agent data/logs.
$enrollmentPath = Join-Path $basePath 'SessionEnrollment'
Protect-Directory $enrollmentPath $false
$traverseAcl = Get-Acl -LiteralPath $enrollmentPath
$traverseAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($userSid, 'Traverse', 'Allow'))
Set-Acl -LiteralPath $enrollmentPath -AclObject $traverseAcl
$userPath = Join-Path $enrollmentPath $userSid.Value
Protect-Directory $userPath $true
$sessionPath = Join-Path $userPath 'session.env'
if (Test-Path -LiteralPath $sessionPath) { throw 'Existing session config will not be overwritten.' }
$original = [IO.File]::ReadAllText($configPath)
$keyMatches = [regex]::Matches($original, '(?m)^\s*(?:export\s+)?AGENT_SESSION_RELAY_KEY\s*=\s*["'']?([0-9a-fA-F]{64})["'']?\s*$')
$assignmentCount = [regex]::Matches($original, '(?m)^\s*(?:export\s+)?AGENT_SESSION_RELAY_KEY\s*=').Count
if ($assignmentCount -gt 0 -and ($assignmentCount -ne 1 -or $keyMatches.Count -ne 1)) { throw 'Existing key is invalid or ambiguous; refusing rotation.' }
if ($keyMatches.Count -eq 1) { $relayKey = $keyMatches[0].Groups[1].Value }
else {
    $bytes = New-Object byte[] 32
    $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
    $relayKey = [BitConverter]::ToString($bytes).Replace('-', '').ToLowerInvariant()
}
$settings = [ordered]@{ AGENT_SESSION_RELAY_ENABLED='false'; AGENT_SESSION_RELAY_PORT="$Port"; AGENT_SESSION_RELAY_KEY=$relayKey }
$pattern = '^\s*(?:export\s+)?(' + (($settings.Keys | ForEach-Object { [regex]::Escape($_) }) -join '|') + ')\s*='
$lines = @($original -split '\r?\n' | Where-Object { $_ -notmatch $pattern })
$updated = ($lines -join "`n") + "`n"
foreach ($entry in $settings.GetEnumerator()) { $updated += "$($entry.Key)=$($entry.Value)`n" }
$cacheKey = New-SessionSecret
$sessionText = "AGENT_MODE=session`nAGENT_SESSION_RELAY_PORT=$Port`nAGENT_SESSION_RELAY_KEY=$relayKey`nAGENT_OFFLINE_CACHE_KEY=$cacheKey`n"
$journalPath = Join-Path $basePath 'session-operation.json'
if (Test-Path -LiteralPath $journalPath) { throw 'Pending operation; use manage-session.ps1 -Action repair.' }
Write-Protected $journalPath (@{ version=1; action='prepare'; sid=$userSid.Value; port=$Port; service_text=$updated; session_text=$sessionText; final_state='prepared' } | ConvertTo-Json) $false
# Fail closed on interruption; never silently enroll a second principal.
Write-Protected $markerPath (@{ state='preparing'; sid=$userSid.Value } | ConvertTo-Json) $false
Write-Protected $sessionPath $sessionText $true
Write-Protected $configPath $updated $false
Write-Protected $markerPath (@{ state='prepared'; sid=$userSid.Value; config=$sessionPath; port=$Port } | ConvertTo-Json) $false
Remove-Item -LiteralPath $journalPath
Write-Output "Prepared session config: $sessionPath"
Write-Output 'Relay remains disabled. No service, login task or firewall rule was started/changed.'

} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
