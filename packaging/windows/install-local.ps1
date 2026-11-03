[CmdletBinding()]
param([ValidateSet('install','repair','remove')][string]$Action='install',
      [ValidatePattern('^S-1-5-21-[0-9-]+$')][string]$SessionSid,
      [switch]$ServiceOnly)
$ErrorActionPreference='Stop'
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
if (!$SessionSid) {$SessionSid=$identity.User.Value}
$principal=[Security.Principal.WindowsPrincipal]::new($identity)
if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $argsList=@('-NoProfile','-ExecutionPolicy','Bypass','-File',('"'+$PSCommandPath+'"'),'-Action',$Action,'-SessionSid',$SessionSid)
    if ($ServiceOnly) {$argsList+='-ServiceOnly'}
    $child=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') -ArgumentList $argsList -Verb RunAs -WindowStyle Hidden -Wait -PassThru
    if ($child.ExitCode -ne 0) {throw 'Installation failed. Review install-local.log beside this script.'}
    Write-Output 'Operation completed. Dashboard: http://127.0.0.1:18080 — Mi equipo: http://127.0.0.1:9876'
    exit 0
}
Start-Transcript -LiteralPath (Join-Path $PSScriptRoot 'install-local.log') -Append | Out-Null
try {
    $manifest=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'local-package.json') -Raw | ConvertFrom-Json
    if ($manifest.file -notmatch '^Monitor-Agent-[0-9]+\.[0-9]+\.[0-9]+-local-x64\.msi$') {throw 'Unexpected local package manifest.'}
    $package=Join-Path $PSScriptRoot $manifest.file
    $config=Join-Path $env:ProgramData 'ActivityMonitor\.env'
    $existing=Get-Service ActivityMonitor -ErrorAction SilentlyContinue
    if ($Action -eq 'install' -and ($existing -or (Test-Path -LiteralPath $config))) {
        throw 'Existing installation/configuration detected. It was preserved. Use repair only for an existing local pilot; migrate other installations explicitly.'
    }
    if ($Action -ne 'install') {
        if (!(Test-Path -LiteralPath $config)) {throw 'Local configuration not found.'}
        $configuration=[IO.File]::ReadAllText($config)
        if ($configuration -notmatch '(?m)^AGENT_SERVER_URL=http://127\.0\.0\.1:18080\s*$') {throw 'This helper is restricted to the local pilot configuration.'}
    }
    if ($Action -ne 'remove') {
        $ready=Invoke-RestMethod -Uri 'http://127.0.0.1:18080/api/ready' -TimeoutSec 10
        if ($ready.status -ne 'ready') {throw 'Start the local Docker environment first.'}
    }
    & (Join-Path $PSScriptRoot 'maintenance\install-package.ps1') -PackagePath $package -Action $Action -LocalTest -ExpectedSha256 $manifest.sha256
    if ($Action -eq 'remove') {Write-Output 'Agent removed. Identity, keys and pending data retained.';return}
    $maintenance=Join-Path $env:ProgramFiles 'ActivityMonitor\maintenance'
    if (!$ServiceOnly -and $Action -eq 'install') {
        $sid=[Security.Principal.SecurityIdentifier]::new($SessionSid)
        $user=$sid.Translate([Security.Principal.NTAccount]).Value
        & (Join-Path $maintenance 'multi-session.ps1') -Action enroll -SessionUser $user -Port 47660
        & (Join-Path $maintenance 'multi-session.ps1') -Action activate -SessionUser $user
    }
    Start-Service ActivityMonitor
    (Get-Service ActivityMonitor).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
    if (!$ServiceOnly -and $Action -eq 'install') {
        # Explicit limited interactive token: never launch desktop collection as SYSTEM or elevated.
        $taskName='ActivityMonitor-Session-'+$SessionSid
        if (Get-ScheduledTask -TaskName $taskName -TaskPath '\' -ErrorAction SilentlyContinue) {throw 'A session task already exists; it was not overwritten.'}
        $launcher=Join-Path $maintenance 'start-session.ps1'
        $sessionConfig=Join-Path (Join-Path (Join-Path $env:ProgramData 'ActivityMonitor\SessionEnrollment') $SessionSid) 'session.env'
        $arguments='-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File "'+$launcher+'" -BinaryPath "'+(Join-Path $env:ProgramFiles 'ActivityMonitor\activity-monitor-agent.exe')+'" -ConfigPath "'+$sessionConfig+'"'
        $taskAction=New-ScheduledTaskAction -Execute (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') -Argument $arguments
        $taskPrincipal=New-ScheduledTaskPrincipal -UserId $SessionSid -LogonType Interactive -RunLevel Limited
        $settings=New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -RestartCount 5 -RestartInterval (New-TimeSpan -Minutes 1) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
        Register-ScheduledTask -TaskName $taskName -TaskPath '\' -Action $taskAction -Principal $taskPrincipal -Settings $settings -Description 'Local Monitor pilot: immediate desktop session; subsequent logins use the installed dispatcher.' | Out-Null
        Start-ScheduledTask -TaskName $taskName -TaskPath '\'
    }
    Write-Output 'Service started. Allow up to two minutes for initial metrics. Dashboard: http://127.0.0.1:18080; local view: http://127.0.0.1:9876'
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
} finally {Stop-Transcript | Out-Null}
