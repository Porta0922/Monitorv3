#Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('enable','disable')][string]$Action,
    [Parameter(Mandatory)][string]$BinaryPath
)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
    $basePath = Join-Path $env:ProgramData 'ActivityMonitor'
    $markerPath = Join-Path $basePath 'session-enrollment.json'
    $adminSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
    $systemSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
    . (Join-Path $PSScriptRoot 'session-common.ps1')
    Assert-NoReparse $basePath; Assert-AdminControlled $basePath
    Assert-NoReparse $markerPath; Assert-AdminControlled $markerPath
    if (Test-Path -LiteralPath (Join-Path $basePath 'session-operation.json')) { throw 'Repair pending credentials first.' }
    $record = [IO.File]::ReadAllText($markerPath) | ConvertFrom-Json
    $sid = [Security.Principal.SecurityIdentifier]::new($record.sid)
    $null = $sid.Translate([Security.Principal.NTAccount])
    $taskName = 'ActivityMonitor-Session-' + $sid.Value
    $binary = [IO.Path]::GetFullPath($BinaryPath)
    $launcher = Join-Path $PSScriptRoot 'start-session.ps1'
    $sessionConfig = Join-Path (Join-Path (Join-Path $basePath 'SessionEnrollment') $sid.Value) 'session.env'
    foreach ($path in @($binary,$launcher,$sessionConfig)) { if ($path.Contains('"') -or $path.Contains("`n") -or $path.Contains("`r")) { throw 'Invalid task path.' } }
    $powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = '-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + $launcher + '" -BinaryPath "' + $binary + '" -ConfigPath "' + $sessionConfig + '"'
    $existing = Get-ScheduledTask -TaskPath '\' -TaskName $taskName -ErrorAction SilentlyContinue
    if ($existing) {
        if ($existing.Actions.Count -ne 1 -or $existing.Actions[0].Execute -ne $powershell -or $existing.Actions[0].Arguments -ne $arguments -or $existing.Principal.UserId -ne $sid.Value) {
            throw 'Existing task does not match this enrollment and package path; refusing to replace/remove it.'
        }
    }
    if ($Action -eq 'disable') {
        if ($existing) {
            Stop-ScheduledTask -TaskName $taskName -TaskPath '\'
            Unregister-ScheduledTask -TaskName $taskName -TaskPath '\' -Confirm:$false
        }
        Write-Output 'Session autostart removed. Configuration, identity and pending data retained.'
    } else {
        if ($record.state -ne 'active') { throw 'Activate the enrollment before enabling autostart.' }
        foreach ($path in @($binary,(Split-Path -Parent $binary),$launcher,$sessionConfig,(Split-Path -Parent $sessionConfig),$PSScriptRoot)) { Assert-NoReparse $path; Assert-AdminControlled $path }
        $trigger = New-ScheduledTaskTrigger -AtLogOn -User $sid.Value
        $principal = New-ScheduledTaskPrincipal -UserId $sid.Value -LogonType Interactive -RunLevel Limited
        $settings = New-ScheduledTaskSettingsSet -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
        $taskAction = New-ScheduledTaskAction -Execute $powershell -Argument $arguments
        Register-ScheduledTask -TaskName $taskName -TaskPath '\' -Action $taskAction -Trigger $trigger -Principal $principal -Settings $settings -Description 'ActivityMonitor enrolled desktop session. Preserves pending data on removal.' -Force | Out-Null
        Write-Output 'Autostart registered for next interactive login. No agent started.'
    }
} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
