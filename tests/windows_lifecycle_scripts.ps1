# Filesystem operations are real; all service operations below are simulated.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $repo ('target/windows-lifecycle-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
$script:installed = Join-Path $fixture 'agent.exe'
$script:serviceState = 'Running'
$script:deleted = $false
$good = Join-Path $fixture 'good.exe'
$bad = Join-Path $fixture 'bad.exe'
Set-Content -LiteralPath $script:installed -Value original
Set-Content -LiteralPath $good -Value candidate
Set-Content -LiteralPath $bad -Value broken
Set-Content -LiteralPath (Join-Path $fixture 'device_id.json') -Value identity
Set-Content -LiteralPath (Join-Path $fixture 'agent_service_cache.db') -Value pending
Set-Content -LiteralPath (Join-Path $fixture 'agent.env') -Value configuration
function Get-CimInstance { param($ClassName,$Filter) [pscustomobject]@{PathName='"'+$script:installed+'"';State=$script:serviceState} }
function Get-Service {
    param($Name)
    $mock = [pscustomobject]@{Status=$script:serviceState}
    $mock | Add-Member ScriptMethod WaitForStatus { param($status,$timeout) if ($script:serviceState -ne $status) { throw 'Unexpected state' } }
    return $mock
}
function Stop-Service { param($Name,$ErrorAction) $script:serviceState='Stopped' }
function Start-Service {
    param($Name)
    if ((Get-Content -LiteralPath $script:installed -Raw).Contains('broken')) { throw 'Injected startup failure' }
    $script:serviceState='Running'
}
function Start-Sleep { param($Seconds) }
function sc.exe { $script:deleted=$true; $global:LASTEXITCODE=0 }
function Assert-True($value,$message) { if (!$value) { throw $message } }
# Only the test executes copies without #Requires; service commands are mocked above.
$update = [scriptblock]::Create((Get-Content (Join-Path $repo 'packaging/windows/update-service.ps1') -Raw).Replace('#Requires -RunAsAdministrator',''))
$uninstall = [scriptblock]::Create((Get-Content (Join-Path $repo 'packaging/windows/uninstall-service.ps1') -Raw).Replace('#Requires -RunAsAdministrator',''))
$failed=$false
try { & $update -BinaryPath $good -ExpectedSha256 ('0'*64) } catch { $failed=$true }
Assert-True $failed 'Expected hash rejection'
Assert-True ((Get-Content $script:installed).Trim() -eq 'original') 'Hash failure changed binary'
Assert-True ($script:serviceState -eq 'Running') 'Hash failure stopped service'
& $update -BinaryPath $good -ExpectedSha256 (Get-FileHash $good).Hash
Assert-True ((Get-Content $script:installed).Trim() -eq 'candidate') 'Update failed'
$failed=$false
try { & $update -BinaryPath $bad -ExpectedSha256 (Get-FileHash $bad).Hash } catch { $failed=$true }
Assert-True $failed 'Expected startup failure'
Assert-True ((Get-Content $script:installed).Trim() -eq 'candidate') 'Rollback did not restore binary'
Assert-True ($script:serviceState -eq 'Running') 'Rollback did not restore running state'
$script:serviceState='Stopped'
& $update -BinaryPath $good -ExpectedSha256 (Get-FileHash $good).Hash
Assert-True ($script:serviceState -eq 'Stopped') 'Stopped service was started'
$failed=$false
try { & $uninstall -BinaryPath $bad } catch { $failed=$true }
Assert-True ($failed -and !$script:deleted) 'Wrong path removed service'
& $uninstall -BinaryPath $script:installed
Assert-True $script:deleted 'Service not removed'
Assert-True (!(Test-Path $script:installed)) 'Binary not removed'
Assert-True ((Get-Content (Join-Path $fixture 'device_id.json')).Trim() -eq 'identity') 'Identity lost'
Assert-True ((Get-Content (Join-Path $fixture 'agent_service_cache.db')).Trim() -eq 'pending') 'Pending events lost'
Assert-True ((Get-Content (Join-Path $fixture 'agent.env')).Trim() -eq 'configuration') 'Configuration lost'
Write-Output 'PASS: Windows hash rejection, update, rollback, stopped state, path validation, uninstall (SCM simulated)'
