"""Summarize local Windows agent measurements without exposing application data."""
import json, math, sys
from pathlib import Path
folder=Path(sys.argv[1])
rows=[json.loads(line) for line in (folder/'samples.jsonl').read_text(encoding='utf-8-sig').splitlines() if line.strip()]
def percentile(values,p):
 values=sorted(values)
 return values[max(0,math.ceil(len(values)*p)-1)] if values else None
summary={}
groups={phase:[r for r in rows if r['phase']==phase] for phase in ('normal','offline','recovery')}
normal=groups['normal']
first_drained=next((i for i,r in enumerate(normal) if r['connected'] and r['pending'] is not None and r['pending']<=2),None)
if first_drained is not None and first_drained>0:
 groups['backlog_recovery']=normal[:first_drained+1]
 groups['steady']=normal[first_drained+1:]
for phase,rs in groups.items():
 if not rs: continue
 # The first global sample lacks a preceding process-counter snapshot.
 measured=[r for r in rs if r is not rows[0]]
 seconds=sum(r['interval_s'] for r in measured)
 cpu=sum(r['cpu_host_percent']*r['interval_s'] for r in measured)/seconds
 sent=[r['broker_sent_bytes'] for r in rs if r['broker_sent_bytes'] is not None]
 recv=[r['broker_received_bytes'] for r in rs if r['broker_received_bytes'] is not None]
 summary[phase]={
  'samples':len(rs),'pending_first':rs[0]['pending'],'observed_seconds':round(seconds,1),
  'cpu_total_mean_pct':round(cpu,4),
  'cpu_total_p95_pct':round(percentile([r['cpu_host_percent'] for r in measured],.95),4),
  'cpu_total_max_pct':round(max(r['cpu_host_percent'] for r in measured),4),
  'cpu_one_core_mean_pct':round(cpu*rs[0]['logical_cores'],3),
  'working_set_mean_mib':round(sum(r['working_set_mib']*r['interval_s'] for r in measured)/seconds,2),
  'working_set_max_mib':round(max(r['working_set_mib'] for r in rs),2),
  'private_max_mib':round(max(r['private_mib'] for r in rs),2),
  'io_read_mean_kib_s':round(sum(r['process_io_read_bytes'] for r in measured)/seconds/1024,3),
  'io_write_mean_kib_s':round(sum(r['process_io_write_bytes'] for r in measured)/seconds/1024,3),
  'io_read_max_kib_s':round(max(r['process_io_read_bytes']/r['interval_s']/1024 for r in measured),3),
  'io_write_max_kib_s':round(max(r['process_io_write_bytes']/r['interval_s']/1024 for r in measured),3),
  'amqp_sent_kib':round(sum(sent)/1024,3) if sent else None,
  'amqp_received_kib':round(sum(recv)/1024,3) if recv else None,
  'amqp_counter_samples':len(sent),
  'host_cpu_max_pct':max(r['host_cpu_percent'] for r in rs),
  'host_disk_max_kib_s':round(max(r['host_disk_bytes_per_sec'] or 0 for r in rs)/1024,3),
  'host_disk_queue_max':max(r['host_disk_queue'] or 0 for r in rs),
  'pending_max':max(r['pending'] for r in rs if r['pending'] is not None),
  'pending_last':rs[-1]['pending'],
  'connected_last':rs[-1]['connected'],
  'agent_processes_min':min(r['agent_processes'] for r in rs),
  'agent_processes_max':max(r['agent_processes'] for r in rs),
  'descendants_max':max(r['descendants'] for r in rs),
  'collector_p95_seconds':round(percentile([r['collection_seconds'] for r in rs],.95),3),
  'cpu_reference_pass':cpu<1 and percentile([r['cpu_host_percent'] for r in measured],.95)<3,
  'memory_reference_pass':max(r['working_set_mib'] for r in rs)<150,
 }
recovery=[r for r in rows if r['phase']=='recovery']
if recovery:
 first_connected=next((r for r in recovery if r['connected']),None)
 drained=next((r for r in recovery if r['connected'] and r['pending'] is not None and r['pending']<=2),None)
 summary['recovery']['connection_observed_after_first_sample_s']=round(first_connected['elapsed_s']-recovery[0]['elapsed_s'],2) if first_connected else None
 summary['recovery']['queue_le_2_after_first_sample_s']=round(drained['elapsed_s']-recovery[0]['elapsed_s'],2) if drained else None
result={'logical_cores':rows[0]['logical_cores'],'version':sorted(set(r['version'] for r in rows if r['version'])),'phases':summary,'scope':'installed agent service + desktop + visible descendants; short observation, no A/B usability test'}
(folder/'summary.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
print(json.dumps(result,indent=2))
