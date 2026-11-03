# Runs without elevation in each Explorer desktop (console or RDP).
$ErrorActionPreference='Stop'
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$basePath=Join-Path $env:ProgramData 'ActivityMonitor'
$userPath=Join-Path (Join-Path $basePath 'SessionEnrollment') $sid
$flag=Join-Path $userPath 'multi-autostart.enabled'
if (!(Test-Path -LiteralPath $flag)) {exit 0}
foreach ($journal in @('session-operation.json','multi-session-operation.json','package-session-operation.json')) {
    if (Test-Path -LiteralPath (Join-Path $basePath $journal)) {exit 1}
}
$binary=Join-Path (Split-Path -Parent $PSScriptRoot) 'activity-monitor-agent.exe'
$config=Join-Path $userPath 'session.env'
$env:AGENT_MODE='session';$env:AGENT_CONFIG_FILE=$config
for ($attempt=0;$attempt -lt 10;$attempt++) {
    if (!(Test-Path -LiteralPath $flag)) {exit 0}
    & $binary
    $code=$LASTEXITCODE
    if ($code -eq 0) {exit 0}
    Start-Sleep -Seconds 15
}
exit $code
