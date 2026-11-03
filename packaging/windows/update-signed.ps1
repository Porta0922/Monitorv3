#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][string]$BinaryPath)
$ErrorActionPreference='Stop'
$trustPath=Join-Path $env:ProgramData 'ActivityMonitor\release-publisher.txt'
$adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
. (Join-Path $PSScriptRoot 'session-common.ps1')
Assert-NoReparse $trustPath; Assert-AdminControlled $trustPath
Assert-AdminControlled (Split-Path -Parent $trustPath)
$expected=[IO.File]::ReadAllText($trustPath).Trim().Replace(' ','').ToUpperInvariant()
if ($expected -notmatch '^[A-F0-9]{40}$') { throw 'Expected an independently provisioned publisher certificate thumbprint.' }
$stage=Join-Path (Split-Path -Parent $trustPath) ('.signed-' + [guid]::NewGuid().ToString('N') + '.exe')
try {
    Copy-Item -LiteralPath $BinaryPath -Destination $stage
    $signature=Get-AuthenticodeSignature -LiteralPath $stage
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $expected) { throw 'Release signature/publisher rejected.' }
    $hash=(Get-FileHash -LiteralPath $stage -Algorithm SHA256).Hash
    & (Join-Path $PSScriptRoot 'update-service.ps1') -BinaryPath $stage -ExpectedSha256 $hash
} finally { if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage } }
