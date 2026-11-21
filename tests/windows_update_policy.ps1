$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '../packaging/windows/update-policy.ps1')
function Assert($condition,$message) {if (!$condition) {throw $message}}
$releases=@(
    @{tag_name='v3.4.0';draft=$false;prerelease=$false},
    @{tag_name='agent-v0.2.9';draft=$false;prerelease=$false},
    @{tag_name='agent-v0.2.10';draft=$false;prerelease=$true},
    @{tag_name='agent-v9.0.0';draft=$true;prerelease=$false},
    @{tag_name='agent-v0.2.11-rc1';draft=$false;prerelease=$false}
)
Assert ((Select-MonitorRelease $releases candidate).tag_name -eq 'agent-v0.2.10') 'Numeric candidate selection failed'
Assert ((Select-MonitorRelease $releases stable).tag_name -eq 'agent-v0.2.9') 'Stable channel selection failed'
Assert ($null -eq (Select-MonitorRelease @($releases[0]) candidate)) 'Legacy release must never be installed'
$asset=@{name='monitor-agent-0.2.10-windows-x64.msi';state='uploaded';digest=('sha256:'+('a'*64));browser_download_url='https://github.com/Porta0922/Monitorv3/releases/download/agent-v0.2.10/monitor-agent-0.2.10-windows-x64.msi'}
$release=@{tag_name='agent-v0.2.10';assets=@($asset)}
Assert ((Get-MonitorWindowsAsset $release).name -eq $asset.name) 'Valid asset rejected'
foreach ($field in @('digest','browser_download_url','state','name')) {
    $bad=$asset.Clone();$bad[$field]='invalid'
    $rejected=$false
    try {Get-MonitorWindowsAsset @{tag_name='agent-v0.2.10';assets=@($bad)}|Out-Null} catch {$rejected=$true}
    Assert $rejected "Invalid $field accepted"
}
$rejected=$false
try {Get-MonitorWindowsAsset @{tag_name='agent-v0.2.10';assets=@($asset,$asset)}|Out-Null} catch {$rejected=$true}
Assert $rejected 'Ambiguous package accepted'
Write-Output 'PASS: numeric versions, legacy exclusion, channels, draft exclusion, asset origin, digest and ambiguity'

Assert ((Get-MonitorFailureStatus $false) -eq 'check_failed') 'Download failures must remain retryable'
Assert ((Get-MonitorFailureStatus $true) -eq 'failed') 'Installation failures must block automatic retries'
