[CmdletBinding()]
param([int]$NormalSeconds=120,[int]$OfflineSeconds=60,[int]$RecoverySeconds=120,[int]$IntervalSeconds=2,[string]$OutputDirectory='.local-test/performance-0.2.4',[switch]$InterruptLocalBroker)
$ErrorActionPreference='Stop'
if($NormalSeconds -lt 10 -or $RecoverySeconds -lt 10 -or $OfflineSeconds -lt 0 -or $OfflineSeconds -gt 120 -or $IntervalSeconds -lt 1){throw 'Invalid measurement durations'}
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
$out=[IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
if(!$out.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Output must stay inside workspace'}
New-Item -ItemType Directory -Path $out -Force|Out-Null
$sampleFile=Join-Path $out 'samples.jsonl'
if(Test-Path -LiteralPath $sampleFile){throw 'Choose a new output directory; evidence is immutable'}
$cores=[Environment]::ProcessorCount
$prev=@{};$connections=@{};$rows=[Collections.Generic.List[object]]::new()
$broker='monitorv32-local-rabbitmq-1'
$gateway=(& docker inspect $broker --format '{{range .NetworkSettings.Networks}}{{.Gateway}}{{end}}').Trim()
if($LASTEXITCODE -ne 0){throw 'Local pilot broker missing'}
$private=Get-Content -LiteralPath '.local-test/.env' | Where-Object {$_ -match '^LOCAL_RABBIT_PASSWORD='}|Select-Object -First 1
$password=($private -split '=',2)[1].Trim().Trim('"').Trim("'")
$auth=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('monitor_local:'+$password))
$headers=@{Authorization='Basic '+$auth};$password=$null;$private=$null
$timer=[Diagnostics.Stopwatch]::StartNew();$lastAt=$timer.Elapsed.TotalSeconds
$paused=$false;$lastNetwork=-10;$networkError=$null;$latestHost=$null
$phases=@(@{name='normal';seconds=$NormalSeconds},@{name='offline';seconds=$OfflineSeconds},@{name='recovery';seconds=$RecoverySeconds})
try {
 foreach($phase in $phases){
  if($phase.name -eq 'offline'){
   if(!$InterruptLocalBroker -or $OfflineSeconds -eq 0){continue}
   & docker stop --time 10 $broker | Out-Null
   if($LASTEXITCODE -ne 0){throw 'Could not stop local broker'}
   $paused=$true
  }
  if($phase.name -eq 'recovery' -and $paused){
   & docker start $broker | Out-Null
   if($LASTEXITCODE -ne 0){throw 'Could not resume local broker'}
   $paused=$false
  }
  $phaseStart=$timer.Elapsed.TotalSeconds
  Write-Output ('Phase: '+$phase.name)
  while(($timer.Elapsed.TotalSeconds-$phaseStart) -lt $phase.seconds){
   $tick=$timer.Elapsed.TotalSeconds
   $all=@(Get-CimInstance Win32_Process)
   $agents=@($all|Where-Object Name -eq 'activity-monitor-agent.exe')
   $ids=@($agents|ForEach-Object {[int]$_.ProcessId})
   # Include descendants such as transient inventory/osquery processes when visible.
   do {$children=@($all|Where-Object {([int]$_.ParentProcessId -in $ids) -and ([int]$_.ProcessId -notin $ids)});$ids+=@($children|ForEach-Object {[int]$_.ProcessId})}while($children.Count -gt 0)
   $selected=@($all|Where-Object {[int]$_.ProcessId -in $ids})
   $now=$timer.Elapsed.TotalSeconds;$dt=$now-$lastAt;$lastAt=$now
   $cpu=0.0;$read=0.0;$write=0.0;$other=0.0;$ws=0.0;$privateBytes=0.0
   foreach($p in $selected){
    $key=[string]$p.ProcessId+':'+[string]$p.CreationDate
    $current=@{cpu=([double]$p.KernelModeTime+[double]$p.UserModeTime)/1e7;read=[double]$p.ReadTransferCount;write=[double]$p.WriteTransferCount;other=[double]$p.OtherTransferCount}
    # Floating zero is essential: an integer zero can select Math.Max(Int32, Int32).
    if($prev.ContainsKey($key)){$cpu+=[Math]::Max(0.0,$current.cpu-$prev[$key].cpu);$read+=[Math]::Max(0.0,$current.read-$prev[$key].read);$write+=[Math]::Max(0.0,$current.write-$prev[$key].write);$other+=[Math]::Max(0.0,$current.other-$prev[$key].other)}
    $prev[$key]=$current;$ws+=[double]$p.WorkingSetSize;$privateBytes+=[double]$p.PrivatePageCount
   }
   $sent=$null;$received=$null;$connectionCount=$null
   if(!$paused -and $now-$lastNetwork -ge 5){
    $lastNetwork=$now
    try{
     $response=Invoke-RestMethod 'http://127.0.0.1:15675/api/connections' -Headers $headers -TimeoutSec 2
     $cs=@($response | Where-Object { $_.peer_host -eq $gateway })
     $connectionCount=$cs.Count
     if($cs.Count -le 1){
      $sent=0.0;$received=0.0
      foreach($c in $cs){
       $key=[string]$c.name
       if($connections.ContainsKey($key)){$sent+=[Math]::Max(0.0,[double]$c.recv_oct-$connections[$key].sent);$received+=[Math]::Max(0.0,[double]$c.send_oct-$connections[$key].received)}
       elseif($phase.name -ne 'normal'){$sent+=[double]$c.recv_oct;$received+=[double]$c.send_oct}
       $connections[$key]=@{sent=[double]$c.recv_oct;received=[double]$c.send_oct}
      }
     }
     $networkError=$null
    }catch{$networkError='broker management unavailable'}
   }
   $status=$null
   try{$status=Invoke-RestMethod 'http://127.0.0.1:9876/api/local' -TimeoutSec 2}catch{}
   $hostCpu=(Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor -Filter "Name='_Total'").PercentProcessorTime
   $disk=Get-CimInstance Win32_PerfFormattedData_PerfDisk_PhysicalDisk -Filter "Name='_Total'"
   $r=[ordered]@{at=[DateTime]::UtcNow.ToString('o');phase=$phase.name;elapsed_s=$now;interval_s=$dt;logical_cores=$cores;agent_processes=$agents.Count;descendants=$selected.Count-$agents.Count;cpu_host_percent=100*$cpu/$dt/$cores;cpu_one_core_percent=100*$cpu/$dt;working_set_mib=$ws/1MB;private_mib=$privateBytes/1MB;process_io_read_bytes=$read;process_io_write_bytes=$write;process_io_other_bytes=$other;broker_sent_bytes=$sent;broker_received_bytes=$received;broker_connections=$connectionCount;network_error=$networkError;host_cpu_percent=$hostCpu;host_disk_bytes_per_sec=$disk.DiskBytesPersec;host_disk_queue=$disk.CurrentDiskQueueLength;version=$status.version;connected=$status.agent.publisher_available;pending=$status.agent.cache_pending;collection_seconds=$timer.Elapsed.TotalSeconds-$tick}
   $rows.Add([pscustomobject]$r)
   $r|ConvertTo-Json -Compress|Add-Content -LiteralPath $sampleFile -Encoding utf8
   $sleep=[Math]::Max(0.0,$IntervalSeconds-($timer.Elapsed.TotalSeconds-$tick))
   if($sleep -gt 0){Start-Sleep -Milliseconds ([int]($sleep*1000))}
  }
 }
}finally{
 if($paused){& docker start $broker|Out-Null;if($LASTEXITCODE -ne 0){Write-Error 'URGENT: resume monitorv32-local-rabbitmq-1 manually'}}
 @{logical_cores=$cores;normal_seconds=$NormalSeconds;offline_seconds=$OfflineSeconds;recovery_seconds=$RecoverySeconds;interruption_requested=[bool]$InterruptLocalBroker;at=[DateTime]::UtcNow.ToString('o');samples=$rows.Count;network_scope='AMQP application bytes for the sole Docker gateway connection; not physical NIC traffic';io_scope='Process IO counters include files, devices and network; not physical disk bytes';limits='Short-lived child processes between samples may be missed; observer and Docker share host; not an A/B usability benchmark'}|ConvertTo-Json|Set-Content (Join-Path $out 'metadata.json') -Encoding utf8
}
Write-Output ('Measurement saved: '+$out)
