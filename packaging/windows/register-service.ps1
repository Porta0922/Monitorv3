#Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][string]$ConfigPath,
    [switch]$Repair
)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
$agentBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
$agentConfig = (Resolve-Path -LiteralPath $ConfigPath).Path
if ($agentBinary.Contains('"')) { throw 'Invalid binary path' }
$adminSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
. (Join-Path $PSScriptRoot 'session-common.ps1')
foreach ($path in @($agentBinary,$agentConfig,(Split-Path -Parent $agentBinary),(Split-Path -Parent $agentConfig))) {
    Assert-NoReparse $path; Assert-AdminControlled $path
}
if (Test-Path -LiteralPath (Join-Path (Split-Path -Parent $agentConfig) 'session-operation.json')) { throw 'Repair pending session credentials first.' }
$existing = Get-Service -Name ActivityMonitor -ErrorAction SilentlyContinue
if ($existing) {
    if (!$Repair) { throw 'Existing service detected; use -Repair for registration repair.' }
    if ($existing.Status -ne 'Stopped') { throw 'Stop ActivityMonitor before repair.' }
    $registered = Get-CimInstance Win32_Service -Filter "Name='ActivityMonitor'"
    if ($registered.PathName -ne ('"' + $agentBinary + '"')) { throw 'Repair cannot relocate the installed binary.' }
    $oldEnvironment = (Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\ActivityMonitor' -Name Environment -ErrorAction SilentlyContinue).Environment
    $oldConfig = @($oldEnvironment | Where-Object { $_ -like 'AGENT_CONFIG_FILE=*' })
    if ($oldConfig.Count -ne 1 -or $oldConfig[0] -ne ('AGENT_CONFIG_FILE=' + $agentConfig)) { throw 'Repair cannot change or infer the installed configuration path.' }
    if (@($oldEnvironment | Where-Object { $_ -notlike 'AGENT_MODE=*' -and $_ -notlike 'AGENT_CONFIG_FILE=*' }).Count -gt 0) { throw 'Custom service environment requires explicit migration; refusing to discard it.' }
    Set-Service -Name ActivityMonitor -StartupType Automatic
} else {
    New-Service -Name ActivityMonitor -DisplayName 'ActivityMonitor system agent' `
        -BinaryPathName ('"' + $agentBinary + '"') -StartupType Automatic | Out-Null
}
$serviceKey = 'HKLM:\SYSTEM\CurrentControlSet\Services\ActivityMonitor'
New-ItemProperty -LiteralPath $serviceKey -Name Environment -PropertyType MultiString `
    -Value @('AGENT_MODE=service', ('AGENT_CONFIG_FILE=' + $agentConfig)) -Force | Out-Null
& sc.exe failure ActivityMonitor reset= 86400 actions= restart/15000/restart/60000/none/0
if ($LASTEXITCODE -ne 0) { throw 'Failed to configure service recovery; service remains stopped.' }
& sc.exe failureflag ActivityMonitor 1
if ($LASTEXITCODE -ne 0) { throw 'Failed to configure non-crash recovery; service remains stopped.' }
Write-Output 'Service registered and left stopped. Review configuration before Start-Service ActivityMonitor.'

} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
