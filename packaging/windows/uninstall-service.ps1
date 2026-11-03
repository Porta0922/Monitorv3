#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][string]$BinaryPath)
$ErrorActionPreference = 'Stop'
$lifecycleLock = [Threading.Mutex]::new($false, 'Global\ActivityMonitorLifecycle')
$lockAcquired = $false
try {
    try { $lockAcquired = $lifecycleLock.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $lockAcquired = $true }
    if (!$lockAcquired) { throw 'Another lifecycle operation is running.' }
$serviceName = 'ActivityMonitor'
$expected = [IO.Path]::GetFullPath($BinaryPath)
$serviceInfo = Get-CimInstance Win32_Service -Filter "Name='$serviceName'"
if (!$serviceInfo -or $serviceInfo.PathName -notmatch '^"([^"\r\n]+)"$') {
    throw 'Expected a service registered with a quoted binary path and no arguments.'
}
$installed = [IO.Path]::GetFullPath($Matches[1])
if ($installed -ne $expected) { throw 'BinaryPath does not match the registered service.' }
Stop-Service -Name $serviceName
(Get-Service -Name $serviceName).WaitForStatus('Stopped', [TimeSpan]::FromSeconds(30))
& sc.exe delete $serviceName
if ($LASTEXITCODE -ne 0) { throw 'Service removal failed; binary and data retained.' }
if (Test-Path -LiteralPath $installed -PathType Leaf) { Remove-Item -LiteralPath $installed }
Write-Output 'System service removed. Identity, cache, configuration and logs retained.'
Write-Output 'Session tasks are separate; remove only the explicitly enrolled tasks for each user.'

} finally {
    if ($lockAcquired) { $lifecycleLock.ReleaseMutex() }
    $lifecycleLock.Dispose()
}
