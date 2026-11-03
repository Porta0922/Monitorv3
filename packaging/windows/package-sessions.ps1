#Requires -RunAsAdministrator
[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('prepare','restore','remove')][string]$Action)
$ErrorActionPreference='Stop'
$packageLock=[Threading.Mutex]::new($false,'Global\ActivityMonitorLifecycle');$packageHeld=$false
try {
try {$packageHeld=$packageLock.WaitOne(0)} catch [Threading.AbandonedMutexException] {$packageHeld=$true}
if (!$packageHeld) {throw 'Another lifecycle operation is running.'}
$basePath=Join-Path $env:ProgramData 'ActivityMonitor'
$journalPath=Join-Path $basePath 'package-session-operation.json'
$enrollmentPath=$basePath
$adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$utf8=[Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'session-common.ps1')
Protect-Directory $basePath $false
foreach ($name in @('session-operation.json','multi-session-operation.json')) {
    if (Test-Path -LiteralPath (Join-Path $basePath $name)) {throw 'Repair credential operations before package changes.'}
}
if ($Action -eq 'prepare') {
    if (Test-Path -LiteralPath $journalPath) {throw 'Pending package journal: restore or remove explicitly first.'}
    $flags=@();$tasks=@()
    $root=Join-Path $basePath 'SessionEnrollment'
    if (Test-Path -LiteralPath $root) {
        Assert-NoReparse $root;Assert-AdminControlled $root
        foreach ($directory in Get-ChildItem -LiteralPath $root -Directory) {
            if ($directory.Name -notmatch '^S-1-5-21-[0-9-]+$') {continue}
            Assert-NoReparse $directory.FullName;Assert-AdminControlled $directory.FullName
            $flag=Join-Path $directory.FullName 'multi-autostart.enabled'
            if (Test-Path -LiteralPath $flag) {Assert-NoReparse $flag;Assert-AdminControlled $flag;$flags+= $directory.Name}
        }
    }
    foreach ($task in Get-ScheduledTask -TaskPath '\' -ErrorAction SilentlyContinue | Where-Object {$_.TaskName -like 'ActivityMonitor-Session-*'}) {
        if ($task.Actions.Count -ne 1 -or $task.Actions[0].Arguments -notlike '*start-session.ps1*') {throw 'Conflicting session task; resolve before packaging.'}
        $tasks+=@{name=$task.TaskName;enabled=[bool]$task.Settings.Enabled}
    }
    Write-Protected $journalPath (@{version=1;flags=$flags;tasks=$tasks}|ConvertTo-Json -Depth 5) $false
    foreach ($sid in $flags) {Remove-Item -LiteralPath (Join-Path (Join-Path $root $sid) 'multi-autostart.enabled')}
    foreach ($task in $tasks) {Disable-ScheduledTask -TaskName $task.name -TaskPath '\'|Out-Null;Stop-ScheduledTask -TaskName $task.name -TaskPath '\'}
    $service=Get-Service ActivityMonitor -ErrorAction SilentlyContinue
    if ($service -and $service.Status -ne 'Stopped') {Stop-Service ActivityMonitor; $service.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))}
    $binary=Join-Path $env:ProgramFiles 'ActivityMonitor\activity-monitor-agent.exe'
    foreach ($process in Get-Process -ErrorAction Stop) {
        if ($process.ProcessName -eq 'activity-monitor-agent') {
            if ($process.Path -ne $binary) {throw 'A legacy agent uses another path; migrate it before MSI deployment.'}
            # Retained SQLite WAL is the durable boundary; never delete its files on termination.
            $process.Kill();if (!$process.WaitForExit(20000)) {throw 'Desktop process did not stop.'}
        }
    }
} else {
    if (!(Test-Path -LiteralPath $journalPath)) {return}
    Assert-NoReparse $journalPath;Assert-AdminControlled $journalPath
    $plan=[IO.File]::ReadAllText($journalPath)|ConvertFrom-Json
    if ($plan.version -ne 1) {throw 'Unknown package journal.'}
    if ($Action -eq 'restore') {
        foreach ($sid in $plan.flags) {
            if ($sid -notmatch '^S-1-5-21-[0-9-]+$') {throw 'Invalid journal principal.'}
            $userSid=[Security.Principal.SecurityIdentifier]::new($sid)
            $directory=Join-Path (Join-Path $basePath 'SessionEnrollment') $sid
            Assert-NoReparse $directory;Assert-AdminControlled $directory
            Write-Protected (Join-Path $directory 'multi-autostart.enabled') "enabled`n" $true
        }
        foreach ($task in $plan.tasks) {
            if ($task.name -notmatch '^ActivityMonitor-Session-S-1-5-21-[0-9-]+$') {throw 'Invalid journal task.'}
            if ($task.enabled) {Enable-ScheduledTask -TaskName $task.name -TaskPath '\'|Out-Null}
        }
    } else {
        foreach ($task in $plan.tasks) {
            if ($task.name -notmatch '^ActivityMonitor-Session-S-1-5-21-[0-9-]+$') {throw 'Invalid journal task.'}
            Unregister-ScheduledTask -TaskName $task.name -TaskPath '\' -Confirm:$false
        }
    }
    Remove-Item -LiteralPath $journalPath
}

} finally {if ($packageHeld) {$packageLock.ReleaseMutex()};$packageLock.Dispose()}
