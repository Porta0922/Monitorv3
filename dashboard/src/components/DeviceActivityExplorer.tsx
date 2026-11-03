import {useMemo,useState} from 'react';
import {useQueryState} from '../hooks/useQueryState';
import type {HourGroup} from '../api/desktop';
import {compactDuration as duration, applicationLabel} from '../api/desktop';
export function DeviceActivityExplorer({groups}:{groups:HourGroup[]}) {
 const [query,setQuery]=useQueryState('activity-q');const [hourValue,setHour]=useQueryState('hour','all');const hour=/^(?:[0-9]|1[0-9]|2[0-3])$/.test(hourValue)?hourValue:'all';const [page,setPage]=useState(0);const [limit,setLimit]=useState<Record<string,number>>({});
 const apps=useMemo(()=>{
  const collected=new Map<string,{name:string;seconds:number;windows:Map<string,number>}>();
  for(const group of groups) {if(hour!=='all'&&group.hour!==Number(hour))continue;
   for(const app of group.programs)for(const win of app.windows??[]) {
    if(query&&!`${app.app} ${win.title}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()))continue;
    let row=collected.get(app.app);if(!row){row={name:app.app,seconds:0,windows:new Map()};collected.set(app.app,row);}
    row.seconds+=win.seconds;row.windows.set(win.title,(row.windows.get(win.title)??0)+win.seconds);
   }
  }
  return [...collected.values()].sort((a,b)=>b.seconds-a.seconds||a.name.localeCompare(b.name));
 },[groups,hour,query]);
 const total=apps.reduce((sum,a)=>sum+a.seconds,0);const max=Math.max(1,...groups.map(g=>g.programs.reduce((s,a)=>s+a.seconds,0)));
 const lastPage=Math.max(0,Math.ceil(apps.length/8)-1);const currentPage=Math.min(page,lastPage);
 return <section className="panel activity-explorer"><div className="panel-header"><div><h2>¿En qué se trabajó?</h2><p className="muted">Tiempo reportado por aplicación y ventana; no equivale a horas de encendido.</p></div><span className="muted">{apps.length} aplicaciones · {duration(total)}</span></div>
 <div className="activity-hours" aria-label="Filtrar actividad por hora UTC"><button className="button button-small" aria-pressed={hour==='all'} onClick={()=>{setHour('all');setPage(0);}}>Todo el día (UTC)</button>{groups.map(g=><button key={g.hour} className="hour-choice" aria-pressed={hour===String(g.hour)} title={`${g.label}: ${duration(g.programs.reduce((s,a)=>s+a.seconds,0))}`} onClick={()=>{setHour(String(g.hour));setPage(0);}}><span className="hour-track"><i style={{height:`${Math.max(4,g.programs.reduce((s,a)=>s+a.seconds,0)/max*100)}%`}}/></span><span>{String(g.hour).padStart(2,'0')}:00</span></button>)}</div>
 <div className="activity-search"><label htmlFor="activity-search">Buscar aplicación o título de ventana</label><input id="activity-search" type="search" placeholder="Ej.: Excel, informe, navegador…" value={query} onChange={e=>{setQuery(e.target.value);setPage(0);}}/>{(query||hour!=='all')&&<button className="text-button" onClick={()=>{setQuery('');setHour('all');setPage(0);}}>Limpiar filtros</button>}</div>
 {!apps.length?<div className="empty-state"><h3>{groups.length?'Sin coincidencias':'Sin actividad para esta fecha'}</h3><p>{groups.length?'Prueba otro término u otra hora.':'Selecciona otro día o espera el primer reporte del agente.'}</p></div>:<div className="activity-app-list">{apps.slice(currentPage*8,currentPage*8+8).map(app=><details key={app.name} className="activity-app"><summary><span className="app-monogram" aria-hidden="true">{app.name.split(/[\\/]/).pop()?.slice(0,1).toUpperCase()}</span><span className="app-summary"><strong title={app.name}>{applicationLabel(app.name)}</strong><small>{app.windows.size} ventanas · ver detalle</small></span><span className="app-share"><i style={{width:`${total?app.seconds/total*100:0}%`}}/></span><strong className="app-time">{duration(app.seconds)}</strong></summary><div className="window-list">{[...app.windows].sort((a,b)=>b[1]-a[1]).slice(0,limit[app.name]??10).map(([title,seconds])=><div className="window-row" key={title}><span>{title}</span><strong>{duration(seconds)}</strong></div>)}{app.windows.size>(limit[app.name]??10)&&<button className="button button-small" onClick={()=>setLimit(v=>({...v,[app.name]:(v[app.name]??10)+10}))}>Mostrar más ventanas</button>}</div></details>)}</div>}
 <div className="table-footer"><span>{apps.length?`${currentPage*8+1}–${Math.min((currentPage+1)*8,apps.length)} de ${apps.length} aplicaciones`:'0 aplicaciones'}</span><div><button className="button button-small" disabled={!currentPage} onClick={()=>setPage(currentPage-1)}>Anterior</button><button className="button button-small" disabled={currentPage>=lastPage} onClick={()=>setPage(currentPage+1)}>Siguiente</button></div></div></section>;
}
