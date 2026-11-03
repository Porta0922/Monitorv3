[CmdletBinding()]
param([ValidateSet('start','stop','status')][string]$Action='start')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (!(Test-Path -LiteralPath '.local-test/.env')) {throw 'Missing private local pilot configuration: .local-test/.env'}
    $compose=@('compose','--project-name','monitorv32-local','--env-file','.local-test/.env','-f','docker-compose.yml','-f','docker-compose.local.yml')
    if ($Action -eq 'start') {& docker @compose up -d}
    elseif ($Action -eq 'stop') {& docker @compose stop}
    else {& docker @compose ps}
    if ($LASTEXITCODE -ne 0) {throw 'Docker operation failed. Check that Docker Desktop is running.'}
} finally {Pop-Location}
