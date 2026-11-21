#Requires -RunAsAdministrator
[CmdletBinding()]
param([ValidateSet('register','check','remove')][string]$Action='check',[switch]$AllowUnsigned)
$ErrorActionPreference='Stop'
$taskName='ActivityMonitor-Updates'
$adminSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
$systemSid=[Security.Principal.SecurityIdentifier]::new('S-1-5-18')
. (Join-Path $PSScriptRoot 'session-common.ps1')
. (Join-Path $PSScriptRoot 'update-policy.ps1')
$base=Join-Path $env:ProgramData 'ActivityMonitor'
$directory=Join-Path $base 'Updates'
$binary=Join-Path $env:ProgramFiles 'ActivityMonitor\activity-monitor-agent.exe'
$installedScript=Join-Path $env:ProgramFiles 'ActivityMonitor\maintenance\automatic-updates.ps1'
if ($Action -eq 'remove') {
    Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue | Unregister-ScheduledTask -Confirm:$false
    return
}
Assert-NoReparse $base;Assert-AdminControlled $base
Protect-Directory $directory $false
$configPath=Join-Path $directory 'policy.json'
if ($Action -eq 'register') {
    Assert-NoReparse $installedScript;Assert-AdminControlled $installedScript
    # Preserve an existing administrator policy. A signed install never silently
    # downgrades its trust policy; unsigned bootstrap requires an explicit flag.
    if (!(Test-Path -LiteralPath $configPath)) {
        [IO.File]::WriteAllText($configPath,(@{schema=1;enabled=$true;allow_unsigned=[bool]$AllowUnsigned;channel='candidate'}|ConvertTo-Json))
    }
    $arguments='-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "'+$installedScript+'" -Action check'
    $actionSpec=New-ScheduledTaskAction -Execute (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') -Argument $arguments
    $trigger=New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(5) -RepetitionInterval (New-TimeSpan -Hours 6)
    $trigger.RandomDelay='PT30M'
    $settings=New-ScheduledTaskSettingsSet -Hidden -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 30) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    $principal=New-ScheduledTaskPrincipal -UserId 'S-1-5-18' -LogonType ServiceAccount -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $actionSpec -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null
    return
}
Assert-NoReparse $configPath;Assert-AdminControlled $configPath
$policy=Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
if ($policy.schema -ne 1 -or !$policy.enabled) {return}
if ($policy.channel -notin @('candidate','stable')) {throw 'Invalid update channel.'}
$lock=[Threading.Mutex]::new($false,'Global\ActivityMonitorUpdates');$held=$false;$client=$null;$work=$null;$installAttempted=$false
try {
    try {$held=$lock.WaitOne(0)} catch [Threading.AbandonedMutexException] {$held=$true}
    if (!$held) {return}
    # Do not repeatedly retry a failed candidate or recreate an interrupted install.
    $statusPath=Join-Path $directory 'status.json'
    $previous=$null
    if (Test-Path -LiteralPath $statusPath) {Assert-NoReparse $statusPath;Assert-AdminControlled $statusPath;$previous=Get-Content $statusPath -Raw|ConvertFrom-Json}
    Add-Type -AssemblyName System.Net.Http
    [Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12
    $handler=[Net.Http.HttpClientHandler]::new();$handler.AllowAutoRedirect=$false
    $client=[Net.Http.HttpClient]::new($handler);$client.Timeout=[TimeSpan]::FromMinutes(3)
    $client.DefaultRequestHeaders.UserAgent.ParseAdd('Monitor-Agent-Updater/0.2')
    function Read-Remote([uri]$Uri,[long]$Limit) {
        for ($redirect=0;$redirect -lt 5;$redirect++) {
            if ($Uri.Scheme -ne 'https' -or $Uri.UserInfo -or $Uri.Host -notin @('api.github.com','github.com','release-assets.githubusercontent.com','objects.githubusercontent.com')) {throw 'Untrusted update URL.'}
            $response=$client.GetAsync($Uri,[Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
            try {
                if ([int]$response.StatusCode -in @(301,302,303,307,308)) {$Uri=[uri]::new($Uri,$response.Headers.Location);continue}
                $response.EnsureSuccessStatusCode()|Out-Null
                if ($response.Content.Headers.ContentLength -gt $Limit) {throw 'Update response exceeds limit.'}
                $stream=$response.Content.ReadAsStreamAsync().GetAwaiter().GetResult();$memory=[IO.MemoryStream]::new()
                try {
                    $buffer=New-Object byte[] 65536
                    while (($count=$stream.Read($buffer,0,$buffer.Length)) -gt 0) {
                        if ($memory.Length+$count -gt $Limit) {throw 'Update response exceeds limit.'}
                        $memory.Write($buffer,0,$count)
                    }
                    return ,$memory.ToArray()
                } finally {$stream.Dispose();$memory.Dispose()}
            } finally {$response.Dispose()}
        }
        throw 'Too many update redirects.'
    }
    $releases=[Text.Encoding]::UTF8.GetString((Read-Remote 'https://api.github.com/repos/Porta0922/Monitorv3/releases?per_page=100' 8388608))|ConvertFrom-Json
    $release=Select-MonitorRelease $releases $policy.channel
    if (!$release) {return}
    $version=$release.tag_name.Substring(7)
    $start=[Diagnostics.ProcessStartInfo]::new()
    $start.FileName=$binary;$start.Arguments='--version-json';$start.UseShellExecute=$false
    $start.CreateNoWindow=$true;$start.RedirectStandardOutput=$true
    $process=[Diagnostics.Process]::Start($start)
    try {
        if (!$process.WaitForExit(15000)) {$process.Kill();throw 'Agent version check timed out.'}
        if ($process.ExitCode -ne 0) {throw 'Cannot read installed agent version.'}
        $installed=$process.StandardOutput.ReadToEnd()|ConvertFrom-Json
    } finally {$process.Dispose()}
    if ($installed.os -ne 'windows' -or $installed.arch -ne 'x86_64' -or $installed.version -notmatch '^\d+\.\d+\.\d+$') {throw 'Unsupported installed agent.'}
    if ([version]$version -le [version]$installed.version) {return}
    if ($previous.version -eq $version -and $previous.status -in @('installing','failed')) {return}
    $work=Join-Path $directory ([guid]::NewGuid().ToString('N'));Protect-Directory $work $false
    $name="monitor-agent-$version-windows-x64.msi"
    $asset=Get-MonitorWindowsAsset $release
    $expectedUrl=$asset.browser_download_url
    $package=Join-Path $work $name
    [IO.File]::WriteAllBytes($package,(Read-Remote $expectedUrl 134217728))
    $hash=$asset.digest.Substring(7)
    if ((Get-FileHash $package -Algorithm SHA256).Hash -ne $hash) {throw 'Update digest mismatch.'}
    $installAttempted=$true
    @{version=$version;status='installing';at=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json|Set-Content $statusPath -Encoding utf8
    # Snapshot the trusted installed coordinator before MSI replaces its files.
    foreach ($file in @('install-package.ps1','package-sessions.ps1','session-common.ps1')) {
        $source=Join-Path $PSScriptRoot $file;Assert-NoReparse $source;Assert-AdminControlled $source
        Copy-Item -LiteralPath $source -Destination (Join-Path $work $file)
    }
    $arguments=@{PackagePath=$package;Action='install'}
    if ($policy.allow_unsigned -eq $true) {$arguments.LocalTest=$true;$arguments.ExpectedSha256=$hash}
    & (Join-Path $work 'install-package.ps1') @arguments
    Start-Service ActivityMonitor
    (Get-Service ActivityMonitor).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
    $verify=[Diagnostics.Process]::Start($start)
    try {
        if (!$verify.WaitForExit(15000)) {$verify.Kill();throw 'Updated version check timed out.'}
        $actual=$verify.StandardOutput.ReadToEnd()|ConvertFrom-Json
        if ($verify.ExitCode -ne 0 -or $actual.version -ne $version) {throw 'Installed version differs from release.'}
    } finally {$verify.Dispose()}
    @{version=$version;status='installed' ;at=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json|Set-Content $statusPath -Encoding utf8
} catch {
    if ($held) {
        # No URLs, credentials or exception details in the retained status.
        @{version=$version;status=(Get-MonitorFailureStatus $installAttempted);at=[DateTime]::UtcNow.ToString('o')}|ConvertTo-Json|Set-Content (Join-Path $directory 'status.json') -Encoding utf8
    }
    throw
} finally {if ($client) {$client.Dispose()};if ($held) {$lock.ReleaseMutex()};$lock.Dispose()}
