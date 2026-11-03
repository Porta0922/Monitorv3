"use strict";
const el=id=>document.getElementById(id);
const text=(id,value)=>{el(id).textContent=value;};
const number=value=>typeof value==='number'&&Number.isFinite(value);
const percent=value=>number(value)?`${value.toFixed(1)} %`:'—';
const bytes=value=>{if(!number(value))return '—';const units=['B','KiB','MiB','GiB','TiB'];let i=0;while(value>=1024&&i<units.length-1){value/=1024;i++;}return `${value.toFixed(i?1:0)} ${units[i]}`;};
const labels={window_activity:'Actividad de ventanas',resource_logger:'Recursos del equipo',inventory:'Inventario',health_reporter:'Estado del agente',heartbeat:'Señal de actividad',session_relay:'Sesión de escritorio',retry_synchronizer:'Envíos pendientes',reconnector:'Conexión',local_web:'Vista local',remote_policy:'Configuración central',command_channel:'Canal administrativo',usb_detector:'Dispositivos USB',usb_copy:'Actividad USB',wifi_history:'Redes WiFi',running_apps:'Aplicaciones',heatmap:'Resumen de entrada',security_osquery:'Eventos de seguridad'};
let history=[],lastSample=null,inFlight=false;
function table(id,rows){const body=el(id);body.replaceChildren();for(const cells of rows){const tr=document.createElement('tr');for(const value of cells){const td=document.createElement('td');td.textContent=String(value);tr.append(td);}body.append(tr);}}
function chart(id,field){let path='',drawing=false;history.forEach((sample,i)=>{const value=sample[field];if(!number(value)){drawing=false;return;}const x=history.length===1?0:i/(history.length-1)*600;const y=130-Math.max(0,Math.min(100,value))*1.2;path+=`${drawing?'L':'M'}${x.toFixed(1)},${y.toFixed(1)} `;drawing=true;});el(id).setAttribute('d',path);}
async function refresh(){if(inFlight)return;inFlight=true;el('refresh').disabled=true;const controller=new AbortController();const timer=setTimeout(()=>controller.abort(),5000);try{
 const response=await fetch('/api/local',{cache:'no-store',signal:controller.signal});if(!response.ok)throw new Error('local unavailable');const data=await response.json();
 const metrics=data.resources?.data??{},agent=data.agent,stamp=data.resources?.observed_at;
 text('hostname',data.hostname||'Mi equipo');document.title=`${data.hostname||'Mi equipo'} · Monitor`;
 text('platform',[metrics.hardware?.os_version,metrics.hardware?.cpu_model].filter(Boolean).join(' · ')||'Información del sistema todavía no disponible');
 const age=stamp?(Date.now()-Date.parse(stamp))/1000:Infinity;const stale=!Number.isFinite(age)||age>180;
 el('connection').className=`notice${stale?' warning':''}`;text('connection',stale?'Agente local disponible. Los recursos no tienen una muestra reciente; pueden estar pausados o todavía iniciándose.':'Vista local actualizada. Solo muestra información de este equipo.');
 text('cpu',percent(metrics.cpu_percent));text('cpu-note',stale?'Muestra ausente o desactualizada':number(metrics.cpu_percent)?'Uso total del procesador':'Esperando el siguiente intervalo');
 text('memory',percent(metrics.memory_percent));text('memory-note',`${bytes(metrics.memory_used_bytes)} de ${bytes(metrics.memory_total_bytes)}`);
 text('uptime',number(metrics.uptime_seconds)?`${Math.floor(metrics.uptime_seconds/3600)} h ${Math.floor(metrics.uptime_seconds/60)%60} min`:'—');
 text('pending',number(agent?.cache_pending)?agent.cache_pending.toLocaleString():'—');text('channel',!agent?'Sin estado todavía':agent.publisher_available?'Canal de envío disponible':'Sin canal de envío disponible');
 text('sample',stamp?`Muestra: ${new Date(stamp).toLocaleString()}${stale?' · desactualizada':''}`:'Sin muestra disponible');
 if(stamp&&stamp!==lastSample){lastSample=stamp;history.push({cpu:metrics.cpu_percent,memory:metrics.memory_percent});history=history.slice(-60);chart('cpu-line','cpu');chart('memory-line','memory');}
 table('disks',(metrics.disks??[]).map(d=>[d.mount,percent(d.used_percent),bytes(d.available_bytes)]));el('disks-empty').hidden=!!metrics.disks?.length;
 table('network',(metrics.network??[]).map(n=>[n.name,number(n.receive_bytes_per_second)?`${bytes(n.receive_bytes_per_second)}/s`:'Esperando muestra',number(n.transmit_bytes_per_second)?`${bytes(n.transmit_bytes_per_second)}/s`:'Esperando muestra']));
 text('version',`Agente ${data.version}`);text('policy',!agent?'Estado todavía no disponible.':agent.monitoring_paused?'La supervisión está pausada por la configuración administrativa.':'La supervisión sigue la configuración del área de TI.');
 text('input',data.capabilities?.input_available?'Entrada disponible para la sesión actual.':data.capabilities?.desktop==='none'?'El agente del sistema no captura entrada de escritorio.':'La captura de entrada no está disponible o aún no recibió eventos.');
 table('tasks',(agent?.tasks??[]).map(t=>[labels[t.name]||t.name,t.has_error?'Requiere atención':t.running?'En ejecución':'Detenido',t.restart_count]));
}catch{el('connection').className='notice error';text('connection','No se pudo conectar con el agente local. Los valores visibles pueden estar desactualizados. Se reintentará automáticamente.');}finally{clearTimeout(timer);inFlight=false;el('refresh').disabled=false;}}
el('refresh').addEventListener('click',refresh);document.addEventListener('visibilitychange',()=>{if(!document.hidden)void refresh();});setInterval(()=>{if(!document.hidden)void refresh();},10000);void refresh();
