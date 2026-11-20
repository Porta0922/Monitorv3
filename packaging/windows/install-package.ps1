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
    # SecureRepair resolves the registered source basename. A fresh random MSI
    # basename breaks maintenance when the original temporary source was removed.
    $installer=New-Object -ComObject WindowsInstaller.Installer
    $database=$null;$view=$null;$record=$null
    try {
        $database=$installer.OpenDatabase($stage,0)
        $view=$database.OpenView('SELECT `Value` FROM `Property` WHERE `Property` = ''ProductCode''')
        $view.Execute();$record=$view.Fetch();$productCode=$record.StringData(1)
        $packageName=[IO.Path]::GetFileName($path);$installed=$false
        foreach ($product in $installer.ProductsEx($productCode,'',4)) {
            $packageName=$product.SourceListInfo('PackageName');$installed=$true
        }
        if ($Action -eq 'repair' -and !$installed) {throw 'The supplied MSI product is not installed.'}
        if (!$packageName -or $packageName.Length -gt 255 -or [IO.Path]::GetFileName($packageName) -ne $packageName -or $packageName.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0 -or [IO.Path]::GetExtension($packageName) -ne '.msi') {
            throw 'Invalid registered package source name.'
        }
    } finally {
        if ($view) {$view.Close()}
        foreach ($com in @($record,$view,$database,$installer)) {if ($com) {[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($com)}}
    }
    $sourcePackage=Join-Path $recovery $packageName
    Move-Item -LiteralPath $stage -Destination $sourcePackage
    $stage=$sourcePackage
    Copy-Item -LiteralPath $coordinator -Destination (Join-Path $recovery 'package-sessions.ps1')
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'session-common.ps1') -Destination (Join-Path $recovery 'session-common.ps1')
    $coordinator=Join-Path $recovery 'package-sessions.ps1'
    $wasRunning=(Get-Service ActivityMonitor -ErrorAction SilentlyContinue).Status -eq 'Running'
    & $coordinator -Action prepare
    # /f ignores command-line properties, including our coordination guard.
    # Repair through /i so the launch condition remains enforced.
    $verb=if ($Action -eq 'remove') {'/x'} else {'/i'}
    $installerLog=Join-Path $recovery 'installer.log'
    $installerArguments=@($verb,('"'+$stage+'"'),'/qn','/norestart','MONITOR_COORDINATED=1','/L*v',('"'+$installerLog+'"'))
    if ($Action -eq 'repair') {$installerArguments+=@('REINSTALL=ALL','REINSTALLMODE=amus')}
    $process=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\msiexec.exe') -ArgumentList $installerArguments -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin @(0,3010)) {
        & $coordinator -Action restore
        throw "Windows Installer failed: $($process.ExitCode). Previous autostart restored; services remain stopped."
    }
    if ($Action -eq 'remove') {
        & $coordinator -Action remove
        Get-ScheduledTask -TaskName 'ActivityMonitor-Updates' -ErrorAction SilentlyContinue | Unregister-ScheduledTask -Confirm:$false
    } else {
        & $coordinator -Action restore -ResumeSessions
        if ($wasRunning) {Start-Service ActivityMonitor}
        $updates=Join-Path $env:ProgramFiles 'ActivityMonitor\maintenance\automatic-updates.ps1'
        if (Test-Path -LiteralPath $updates) {& $updates -Action register -AllowUnsigned:$LocalTest}
    }
    Write-Output "Installer completed: $($process.ExitCode). Configuration/data retained; review configuration before starting."
} catch {
    # Preparation can fail after writing its journal and disabling autostart.
    # Restore that journal too, not only failures returned by msiexec.
    if ($held -and $coordinator -and (Test-Path -LiteralPath $coordinator)) {
        try { & $coordinator -Action restore } catch { Write-Warning 'Session recovery remains pending; run package-sessions.ps1 -Action restore as administrator.' }
    }
    throw
} finally { if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage }; if ($held) {$lifecycleLock.ReleaseMutex()};$lifecycleLock.Dispose() }
