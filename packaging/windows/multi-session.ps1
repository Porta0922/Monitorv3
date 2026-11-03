#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('enroll','activate','revoke','rotate','repair')][string]$Action,
      [string]$SessionUser,[int]$Port,[switch]$ImportLegacy)
$ErrorActionPreference='Stop'
$mutex=[Threading.Mutex]::new($false,'Global\ActivityMonitorLifecycle');$held=$false
try {
    try {$held=$mutex.WaitOne(0)} catch [Threading.AbandonedMutexException] {$held=$true}
    if (!$held) {throw 'Another lifecycle operation is running.'}
    if ((Get-Service ActivityMonitor).Status -ne 'Stopped') {throw 'Stop the service and desktop agents first.'}
    foreach ($process in Get-CimInstance Win32_Process) {
        if ($process.Name -in @('activity-monitor-agent.exe','activity-monitor.exe')) {throw 'Stop all desktop agents first.'}
    }
    $basePath=Join-Path $env:ProgramData 'ActivityMonitor'
    $enrollmentPath=Join-Path $basePath 'SessionEnrollment'
    $registryPath=Join-Path $basePath 'Principals'
    $journalPath=Join-Path $basePath 'multi-session-operation.json'
    $adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
    $systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
    $utf8=[Text.UTF8Encoding]::new($false)
    . (Join-Path $PSScriptRoot 'session-common.ps1')
    Assert-NoReparse $basePath;Assert-AdminControlled $basePath
    if (Test-Path -LiteralPath (Join-Path $basePath 'session-operation.json')) {throw 'Repair legacy journal first.'}
    Protect-Directory $enrollmentPath $false;Protect-Directory $registryPath $false
    if (Test-Path -LiteralPath $journalPath) {
        if ($Action -ne 'repair') {throw 'Pending multi-session operation; use repair.'}
        Assert-NoReparse $journalPath;Assert-AdminControlled $journalPath
        $plan=[IO.File]::ReadAllText($journalPath)|ConvertFrom-Json
        if ($plan.version -ne 1) {throw 'Unknown operation journal.'}
        $userSid=[Security.Principal.SecurityIdentifier]::new($plan.sid)
        $null=$userSid.Translate([Security.Principal.NTAccount])
    } else {
        if (!$SessionUser) {throw 'SessionUser is required.'}
        $userSid=([Security.Principal.NTAccount]::new($SessionUser)).Translate([Security.Principal.SecurityIdentifier])
        if ($userSid.Value -notmatch '^S-1-5-21-') {throw 'Choose a local or domain desktop user.'}
        $userPath=Join-Path $enrollmentPath $userSid.Value
        Protect-Directory $userPath $true
        $sessionPath=Join-Path $userPath 'session.env'
        $descriptorPath=Join-Path $registryPath ($userSid.Value+'.json')
        $old=$null
        if (Test-Path -LiteralPath $descriptorPath) {Assert-NoReparse $descriptorPath;Assert-AdminControlled $descriptorPath;$old=[IO.File]::ReadAllText($descriptorPath)|ConvertFrom-Json}
        if ($Action -eq 'enroll' -and $old) {throw 'Principal already registered.'}
        if ($Action -ne 'enroll' -and !$old) {throw 'Principal is not registered.'}
        $current=''
        if (Test-Path -LiteralPath $sessionPath) {
            Assert-NoReparse $sessionPath;Assert-AdminControlled $sessionPath
            if ($Action -eq 'enroll' -and !$ImportLegacy) {throw 'Existing config requires ImportLegacy.'}
            $current=[IO.File]::ReadAllText($sessionPath)
        }
        if ($old) {$Port=[int]$old.port}
        if ($Port -lt 1024 -or $Port -gt 65535) {throw 'Choose a dedicated port from 1024..65535.'}
        $servicePath=Join-Path $basePath '.env';Assert-NoReparse $servicePath;Assert-AdminControlled $servicePath
        $service=[IO.File]::ReadAllText($servicePath)
        if ((Get-SessionSetting $service 'AGENT_SESSION_RELAY_ENABLED' $false) -eq 'true' -and (Get-SessionSetting $service 'AGENT_SESSION_RELAY_PORT' $false) -eq [string]$Port) {throw 'Port is used by legacy relay.'}
        foreach ($file in Get-ChildItem -LiteralPath $registryPath -Filter '*.json' -File) {
            Assert-NoReparse $file.FullName;Assert-AdminControlled $file.FullName
            if ($file.FullName -ne $descriptorPath -and ([IO.File]::ReadAllText($file.FullName)|ConvertFrom-Json).port -eq $Port) {throw 'Port already assigned.'}
        }
        $key=if ($Action -in @('enroll','rotate','revoke')) {New-SessionSecret} else {$old.key}
        $enabled=$Action -eq 'activate' -or ($Action -in @('rotate','repair') -and $old.enabled)
        $settings=[ordered]@{AGENT_MODE='session';AGENT_MULTI_SESSION='true';AGENT_SESSION_RELAY_PORT=$Port;AGENT_SESSION_RELAY_KEY=$key}
        if ($null -eq (Get-SessionSetting $current 'AGENT_OFFLINE_CACHE_KEY' $false)) {
            $cacheKey=Get-SessionSetting $current 'AGENT_SESSION_RELAY_KEY' $false
            if (!$cacheKey) {$cacheKey=New-SessionSecret}
            $settings.AGENT_OFFLINE_CACHE_KEY=$cacheKey
        }
        $serviceSettings=[ordered]@{AGENT_SESSION_REGISTRY=('"'+$registryPath.Replace('\','/')+'"')}
        if ($ImportLegacy) {$serviceSettings.AGENT_SESSION_RELAY_ENABLED='false'}
        $plan=@{version=1;sid=$userSid.Value;enabled=$enabled;service=(Update-SessionEnvironment $service $serviceSettings);
            session=(Update-SessionEnvironment $current $settings);descriptor=@{principal=('sid:'+$userSid.Value);port=$Port;key=$key;enabled=$enabled}}
        Write-Protected $journalPath ($plan|ConvertTo-Json -Depth 5) $false
    }
    $userPath=Join-Path $enrollmentPath $userSid.Value;Protect-Directory $userPath $true
    Write-Protected (Join-Path $userPath 'session.env') $plan.session $true
    Write-Protected (Join-Path $registryPath ($userSid.Value+'.json')) ($plan.descriptor|ConvertTo-Json) $false
    Write-Protected (Join-Path $basePath '.env') $plan.service $false
    $flag=Join-Path $userPath 'multi-autostart.enabled'
    if ($plan.enabled) {Write-Protected $flag "enabled`n" $true}
    elseif (Test-Path -LiteralPath $flag) {Assert-NoReparse $flag;Remove-Item -LiteralPath $flag}
    Remove-Item -LiteralPath $journalPath
    Write-Output 'Principal updated; processes remain stopped. Identity and cache key retained.'
} finally {if ($held) {$mutex.ReleaseMutex()};$mutex.Dispose()}
