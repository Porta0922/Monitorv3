#Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [string]$BinaryPath,
    [ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedSha256,
    [switch]$Recover
)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
$serviceName = 'ActivityMonitor'
$serviceInfo = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
if (!$serviceInfo -or $serviceInfo.PathName -notmatch '^"([^"\r\n]+)"$') {
    throw 'Expected a service registered with a quoted binary path and no arguments.'
}
$installed = [IO.Path]::GetFullPath($Matches[1])
$adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$utf8=[Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'session-common.ps1')
$enrollmentPath=[IO.Path]::GetDirectoryName($installed)
Assert-NoReparse $installed; Assert-AdminControlled $installed; Assert-AdminControlled $enrollmentPath
$journal=Join-Path $enrollmentPath '.update-journal.json'
if (Test-Path -LiteralPath $journal) {
    if (!$Recover) { throw 'Interrupted update found; run with -Recover.' }
    Assert-NoReparse $journal; Assert-AdminControlled $journal
    $plan=[IO.File]::ReadAllText($journal) | ConvertFrom-Json
    if ($plan.installed -ne $installed -or $plan.backup -notmatch '^\.update-[a-f0-9]{32}\.previous$') { throw 'Invalid recovery journal.' }
    $previous=Join-Path $enrollmentPath $plan.backup
    Assert-NoReparse $previous; Assert-AdminControlled $previous
    if ((Get-FileHash -LiteralPath $previous -Algorithm SHA256).Hash -ne $plan.previous_sha256) { throw 'Rollback artifact hash mismatch.' }
    Stop-Service -Name ActivityMonitor
    (Get-Service ActivityMonitor).WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))
    $restore=Join-Path $enrollmentPath ('.restore-' + [guid]::NewGuid().ToString('N'))
    Copy-Item -LiteralPath $previous -Destination $restore
    [IO.File]::Replace($restore,$installed,[NullString]::Value)
    Remove-Item -LiteralPath $journal
    Write-Output 'Previous binary restored; service left stopped; identity and outbox retained.'
    return
}
if ($Recover) { Write-Output 'No interrupted update found.'; return }
if (!$BinaryPath -or !$ExpectedSha256) { throw 'BinaryPath and ExpectedSha256 are required.' }
$source = (Resolve-Path -LiteralPath $BinaryPath).Path
if (!(Test-Path -LiteralPath $installed -PathType Leaf)) { throw 'Installed binary is missing.' }
$stage = Join-Path ([IO.Path]::GetDirectoryName($installed)) ('.update-' + [guid]::NewGuid().ToString('N'))
$backup = $stage + '.previous'
$running = $serviceInfo.State -eq 'Running'
$stopped = $false
$replaced = $false
try {
    Copy-Item -LiteralPath $source -Destination $stage
    if ((Get-FileHash -LiteralPath $stage -Algorithm SHA256).Hash -ne $ExpectedSha256) {
        throw 'SHA256 mismatch; service untouched.'
    }
    Copy-Item -LiteralPath $installed -Destination $backup
    $backupStream=[IO.File]::Open($backup,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try { $backupStream.Flush($true) } finally { $backupStream.Dispose() }
    Write-Protected $journal (@{version=1;installed=$installed;backup=[IO.Path]::GetFileName($backup);previous_sha256=(Get-FileHash -LiteralPath $backup -Algorithm SHA256).Hash;candidate_sha256=$ExpectedSha256;was_running=$running} | ConvertTo-Json) $false
    Stop-Service -Name $serviceName
    (Get-Service -Name $serviceName).WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
    $stopped = $true
    # File.Replace preserves the destination ACL and switches atomically on the same volume.
    [IO.File]::Replace($stage, $installed, [NullString]::Value)
    $replaced = $true
    if ($running) {
        Start-Service -Name $serviceName
        (Get-Service -Name $serviceName).WaitForStatus('Running', [TimeSpan]::FromSeconds(30))
        Start-Sleep -Seconds 3
        if ((Get-Service -Name $serviceName).Status -ne 'Running') { throw 'Updated service failed.' }
    }
    Remove-Item -LiteralPath $journal
    Write-Output 'Local update applied; identity, configuration and pending data retained.'
} catch {
    $failure = $_
    if ($replaced) {
        Stop-Service -Name $serviceName -ErrorAction SilentlyContinue
        (Get-Service -Name $serviceName).WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
        $restore = $stage + '.restore'
        Copy-Item -LiteralPath $backup -Destination $restore
        [IO.File]::Replace($restore, $installed, [NullString]::Value)
    }
    if (Test-Path -LiteralPath $journal) { Remove-Item -LiteralPath $journal }
    if ($running -and $stopped) { Start-Service -Name $serviceName }
    throw $failure
} finally {
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage }
    # Keep rollback artifact if restoration failed; never remove it automatically.
}
# Successful update/rollback copies may be archived explicitly by the administrator.

} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
