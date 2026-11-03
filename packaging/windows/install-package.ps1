#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][string]$PackagePath,[ValidateSet('install','repair','remove')][string]$Action='install',[switch]$LocalTest,[ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSha256)
$ErrorActionPreference='Stop'
$path=(Resolve-Path -LiteralPath $PackagePath).Path
if ([IO.Path]::GetExtension($path) -ne '.msi') { throw 'Expected an MSI package.' }
$trust=Join-Path $env:ProgramData 'ActivityMonitor\release-publisher.txt'
$adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
. (Join-Path $PSScriptRoot 'session-common.ps1')
if ($LocalTest) {
    if (!$ExpectedSha256) {throw 'LocalTest requires an explicit SHA256.'}
    Protect-Directory (Split-Path -Parent $trust) $false
} else {
    Assert-NoReparse $trust;Assert-AdminControlled $trust;Assert-AdminControlled (Split-Path -Parent $trust)
    $expected=[IO.File]::ReadAllText($trust).Trim().Replace(' ','').ToUpperInvariant()
}
$stage=Join-Path (Split-Path -Parent $trust) ('.package-' + [guid]::NewGuid().ToString('N') + '.msi')
$lifecycleLock=[Threading.Mutex]::new($false,'Global\ActivityMonitorLifecycle');$held=$false
try {
    try {$held=$lifecycleLock.WaitOne(0)} catch [Threading.AbandonedMutexException] {$held=$true}
    if (!$held) {throw 'Another lifecycle operation is running.'}
    Copy-Item -LiteralPath $path -Destination $stage
    if ($LocalTest) {
        if ((Get-FileHash -LiteralPath $stage -Algorithm SHA256).Hash -ne $ExpectedSha256) {throw 'Local MSI checksum mismatch.'}
        Write-Output 'LOCAL TEST: checksum-verified package without production signature.'
    } else {
    $signature=Get-AuthenticodeSignature -LiteralPath $stage
    if ($expected -notmatch '^[A-F0-9]{40}$' -or $signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $expected) { throw 'Package signature/publisher rejected.' }
    }
    $coordinator=Join-Path $PSScriptRoot 'package-sessions.ps1'
    # Snapshot coordinator/helper outside INSTALLFOLDER so removal/rollback cannot delete the recovery code.
    $recovery=Join-Path (Split-Path -Parent $trust) ('.package-recovery-'+[guid]::NewGuid().ToString('N'))
    Protect-Directory $recovery $false
    Copy-Item -LiteralPath $coordinator -Destination (Join-Path $recovery 'package-sessions.ps1')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'session-common.ps1') -Destination (Join-Path $recovery 'session-common.ps1')
    $coordinator=Join-Path $recovery 'package-sessions.ps1'
    & $coordinator -Action prepare
    $verb=if ($Action -eq 'remove') {'/x'} elseif ($Action -eq 'repair') {'/fa'} else {'/i'}
    $process=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\msiexec.exe') -ArgumentList @($verb,('"'+$stage+'"'),'/qn','/norestart','MONITOR_COORDINATED=1') -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin @(0,3010)) {
        & $coordinator -Action restore
        throw "Windows Installer failed: $($process.ExitCode). Previous autostart restored; services remain stopped."
    }
    if ($Action -eq 'remove') {& $coordinator -Action remove} else {& $coordinator -Action restore}
    Write-Output "Installer completed: $($process.ExitCode). Configuration/data retained; review configuration before starting."
} finally { if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage }; if ($held) {$lifecycleLock.ReleaseMutex()};$lifecycleLock.Dispose() }
