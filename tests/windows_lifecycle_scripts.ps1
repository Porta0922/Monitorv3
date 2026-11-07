#Requires -RunAsAdministrator
# Filesystem and ACL operations are real; all service operations below are simulated.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $repo ('target/windows-lifecycle-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $fixture | Out-Null
# Model the protected installation directory required by the real updater.
$fixtureAcl = [Security.AccessControl.DirectorySecurity]::new()
$fixtureAcl.SetAccessRuleProtection($true, $false)
$fixtureOwner = [Security.Principal.WindowsIdentity]::GetCurrent().User
$fixtureAcl.SetOwner($fixtureOwner)
foreach ($sid in @($fixtureOwner, [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544'), [Security.Principal.SecurityIdentifier]::new('S-1-5-18'))) {
    $fixtureAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
}
Set-Acl -LiteralPath $fixture -AclObject $fixtureAcl
$global:monitorLifecycleTest_installed = Join-Path $fixture 'agent.exe'
$global:monitorLifecycleTest_serviceState = 'Running'
$global:monitorLifecycleTest_deleted = $false
$good = Join-Path $fixture 'good.exe'
$bad = Join-Path $fixture 'bad.exe'
Set-Content -LiteralPath $global:monitorLifecycleTest_installed -Value original
Set-Content -LiteralPath $good -Value candidate
Set-Content -LiteralPath $bad -Value broken
Set-Content -LiteralPath (Join-Path $fixture 'device_id.json') -Value identity
Set-Content -LiteralPath (Join-Path $fixture 'agent_service_cache.db') -Value pending
Set-Content -LiteralPath (Join-Path $fixture 'agent.env') -Value configuration
function Get-CimInstance { param($ClassName,$Filter) [pscustomobject]@{PathName='"'+$global:monitorLifecycleTest_installed+'"';State=$global:monitorLifecycleTest_serviceState} }
function Get-Service {
    param($Name)
    $mock = [pscustomobject]@{Status=$global:monitorLifecycleTest_serviceState}
    $mock | Add-Member ScriptMethod WaitForStatus { param($status,$timeout) if ($global:monitorLifecycleTest_serviceState -ne $status) { throw 'Unexpected state' } }
    return $mock
}
function Stop-Service { param($Name,$ErrorAction) $global:monitorLifecycleTest_serviceState='Stopped' }
function Start-Service {
    param($Name)
    if ((Get-Content -LiteralPath $global:monitorLifecycleTest_installed -Raw).Contains('broken')) { throw 'Injected startup failure' }
    $global:monitorLifecycleTest_serviceState='Running'
}
function Start-Sleep { param($Seconds) }
function sc.exe { $global:monitorLifecycleTest_deleted=$true; $global:LASTEXITCODE=0 }
function Assert-True($value,$message) { if (!$value) { throw $message } }
# Real script files preserve PSScriptRoot for the shared security helper.
# Only service commands are mocked; filesystem and ACL checks remain real.
Copy-Item -LiteralPath (Join-Path $repo 'packaging/windows/session-common.ps1') -Destination $fixture
foreach ($name in @('update-service.ps1','uninstall-service.ps1')) {
    $source = Get-Content -LiteralPath (Join-Path $repo ('packaging/windows/' + $name)) -Raw
    [IO.File]::WriteAllText((Join-Path $fixture $name), $source.Replace('#Requires -RunAsAdministrator',''))
}
$update = Join-Path $fixture 'update-service.ps1'
$uninstall = Join-Path $fixture 'uninstall-service.ps1'
$failed=$false
try { & $update -BinaryPath $good -ExpectedSha256 ('0'*64) } catch { $failed=$true }
Assert-True $failed 'Expected hash rejection'
Assert-True ((Get-Content $global:monitorLifecycleTest_installed).Trim() -eq 'original') 'Hash failure changed binary'
Assert-True ($global:monitorLifecycleTest_serviceState -eq 'Running') 'Hash failure stopped service'
& $update -BinaryPath $good -ExpectedSha256 (Get-FileHash $good).Hash
Assert-True ((Get-Content $global:monitorLifecycleTest_installed).Trim() -eq 'candidate') 'Update failed'
$failed=$false
try { & $update -BinaryPath $bad -ExpectedSha256 (Get-FileHash $bad).Hash } catch { $failed=$true }
Assert-True $failed 'Expected startup failure'
Assert-True ((Get-Content $global:monitorLifecycleTest_installed).Trim() -eq 'candidate') 'Rollback did not restore binary'
Assert-True ($global:monitorLifecycleTest_serviceState -eq 'Running') 'Rollback did not restore running state'
$global:monitorLifecycleTest_serviceState='Stopped'
& $update -BinaryPath $good -ExpectedSha256 (Get-FileHash $good).Hash
Assert-True ($global:monitorLifecycleTest_serviceState -eq 'Stopped') 'Stopped service was started'
$failed=$false
try { & $uninstall -BinaryPath $bad } catch { $failed=$true }
Assert-True ($failed -and !$global:monitorLifecycleTest_deleted) 'Wrong path removed service'
& $uninstall -BinaryPath $global:monitorLifecycleTest_installed
Assert-True $global:monitorLifecycleTest_deleted 'Service not removed'
Assert-True (!(Test-Path $global:monitorLifecycleTest_installed)) 'Binary not removed'
Assert-True ((Get-Content (Join-Path $fixture 'device_id.json')).Trim() -eq 'identity') 'Identity lost'
Assert-True ((Get-Content (Join-Path $fixture 'agent_service_cache.db')).Trim() -eq 'pending') 'Pending events lost'
Assert-True ((Get-Content (Join-Path $fixture 'agent.env')).Trim() -eq 'configuration') 'Configuration lost'
Write-Output 'PASS: Windows hash rejection, update, rollback, stopped state, path validation, uninstall (SCM simulated)'
