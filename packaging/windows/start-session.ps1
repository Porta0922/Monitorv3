# Launch in the enrolled interactive user's session, never as SYSTEM.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$BinaryPath, [Parameter(Mandatory)][string]$ConfigPath)
$ErrorActionPreference = 'Stop'
$agentBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
$agentConfig = (Resolve-Path -LiteralPath $ConfigPath).Path
$oldMode = $env:AGENT_MODE
$oldConfig = $env:AGENT_CONFIG_FILE
try {
    $env:AGENT_MODE = 'session'
    $env:AGENT_CONFIG_FILE = $agentConfig
    & $agentBinary
    $agentExitCode = $LASTEXITCODE
} finally {
    $env:AGENT_MODE = $oldMode
    $env:AGENT_CONFIG_FILE = $oldConfig
}

exit $agentExitCode
