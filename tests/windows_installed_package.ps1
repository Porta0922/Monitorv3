#Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackagePath,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSha256,
    [Parameter(Mandatory)][string]$ReceiptPath,
    [switch]$LocalTest,
    [switch]$RequireDesktopSession
)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$binary=Join-Path $env:ProgramFiles 'ActivityMonitor/activity-monitor-agent.exe'
$data=Join-Path $env:ProgramData 'ActivityMonitor'
$receipt=[IO.Path]::GetFullPath($ReceiptPath)
if (Test-Path -LiteralPath $receipt) {throw 'Use a new receipt path.'}
if (!(Test-Path -LiteralPath $binary)) {throw 'Install the pilot before testing repair.'}
$package=(Resolve-Path -LiteralPath $PackagePath).Path
if ((Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash -ne $ExpectedSha256) {throw 'Package hash mismatch.'}
$wasRunning=(Get-Service ActivityMonitor).Status -eq 'Running'
$beforeBinary=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
$configs=@{}
Get-ChildItem -LiteralPath $data -File -Force | Where-Object { $_.Name -eq '.env' -or $_.Extension -eq '.env' } | ForEach-Object {
    $configs[$_.FullName]=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
}
if (!$configs.Count) {throw 'No installed configuration found; refusing an unrepresentative repair.'}
$result=@{schema_version=1;success=$false;test='installed_msi_repair';production_approved=$false;unsigned_pilot=[bool]$LocalTest;package_sha256=$ExpectedSha256}
try {
    & (Join-Path $root 'packaging/windows/install-package.ps1') -PackagePath $package -Action repair -ExpectedSha256 $ExpectedSha256 -LocalTest:$LocalTest
    foreach ($path in $configs.Keys) {
        if (!(Test-Path -LiteralPath $path) -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $configs[$path]) {throw 'Installed configuration changed during repair.'}
    }
    if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash -ne $beforeBinary) {throw 'Repair changed installed executable bytes.'}
    $version=& $binary --version-json | ConvertFrom-Json
    $result.version=$version
    $result.configuration_preserved=$true
    $result.binary_preserved=$true
    if ($RequireDesktopSession) {
        $deadline=(Get-Date).AddSeconds(30)
        do {
            $sessions=@(Get-Process activity-monitor-agent -ErrorAction SilentlyContinue | Where-Object {$_.SessionId -gt 0})
            if ($sessions.Count) {break}
            Start-Sleep -Seconds 1
        } while ((Get-Date) -lt $deadline)
        if (!$sessions.Count) {throw 'Desktop collector did not resume after repair.'}
        $result.desktop_session_resumed=$true
    }
    $result.success=$true
} catch {
    $result.failure=$_.Exception.Message
    $result.failure_script=Split-Path -Leaf $_.InvocationInfo.ScriptName
    $result.failure_line=$_.InvocationInfo.ScriptLineNumber
    $latestLog=Get-ChildItem -LiteralPath $data -Directory -Filter '.package-recovery-*' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($latestLog -and (Test-Path -LiteralPath (Join-Path $latestLog.FullName 'installer.log'))) {
        $result.installer_errors=@(Select-String -LiteralPath (Join-Path $latestLog.FullName 'installer.log') -Pattern 'Error [0-9]+','Return value 3','CAQuietExec:.*' | Select-Object -Last 18 | ForEach-Object Line)
    }
    throw
} finally {
    if ($wasRunning) {
        Start-Service ActivityMonitor
        (Get-Service ActivityMonitor).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
    }
    $result.service=(Get-Service ActivityMonitor).Status.ToString()
    $result.completed_at=(Get-Date).ToUniversalTime().ToString('o')
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receipt -Encoding utf8
}
Write-Output 'Installed repair verified; production signing, reboot and uninstall acceptance remain separate.'
