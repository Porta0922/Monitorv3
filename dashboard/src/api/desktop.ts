export interface WindowRow { hour: number; app_name: string; window_title: string; total_seconds: number; count: number }
export interface WindowItem {title:string;seconds:number;duration:string;intervals:number}
export interface ProgramItem {app:string;seconds:number;duration:string;intervals:number;is_idle:boolean;window_count:number;windows:WindowItem[]}
export interface HourGroup {hour:number;label:string;programs:ProgramItem[]}
export function duration(seconds:number) { const n=Math.max(0,Math.floor(seconds));return `${Math.floor(n/3600)} h ${Math.floor(n%3600/60)} min ${n%60} s`; }
export function groupHourlyPrograms(rows:WindowRow[]):HourGroup[] {
 const hours=new Map<number,Map<string,ProgramItem>>();
 for(const row of rows) {
   if(!Number.isInteger(row.hour)||row.hour<0||row.hour>23||typeof row.app_name!=='string'||!Number.isFinite(row.total_seconds)||!Number.isFinite(row.count)) throw new Error('Invalid hourly activity response');
   let apps=hours.get(row.hour);if(!apps){apps=new Map();hours.set(row.hour,apps);}
   let app=apps.get(row.app_name);
   if(!app){app={app:row.app_name,seconds:0,duration:'',intervals:0,is_idle:false,window_count:0,windows:[]};apps.set(row.app_name,app);}
   app.seconds+=row.total_seconds;app.intervals+=row.count;
   app.windows.push({title:row.window_title||'Sin título',seconds:row.total_seconds,duration:duration(row.total_seconds),intervals:row.count});
   app.window_count=app.windows.length;app.duration=duration(app.seconds);
 }
 return [...hours].sort(([a],[b])=>a-b).map(([hour,apps])=>({hour,label:`${String(hour).padStart(2,'0')}:00 – ${String(hour).padStart(2,'0')}:59`,programs:[...apps.values()].sort((a,b)=>b.seconds-a.seconds||a.app.localeCompare(b.app))}));
}

export function compactDuration(seconds:number) { const n=Math.max(0,Math.floor(seconds));return n>=3600?`${Math.floor(n/3600)} h ${Math.floor(n%3600/60)} min`:n>=60?`${Math.floor(n/60)} min ${n%60} s`:`${n} s`; }
export function applicationLabel(name:string) {
 const base=name.split(/[\\/]/).pop()||name;
 const aliases:Record<string,string>={'chrome.exe':'Google Chrome','msedge.exe':'Microsoft Edge','excel.exe':'Excel','winword.exe':'Word','outlook.exe':'Outlook','ms-teams.exe':'Microsoft Teams','explorer.exe':'Explorador de archivos','lockapp.exe':'Pantalla de bloqueo','code.exe':'Visual Studio Code','chatgpt.exe':'ChatGPT','whatsapp.root.exe':'WhatsApp'};
 return aliases[base.toLowerCase()]??base.replace(/\.exe$/i,'');
}
