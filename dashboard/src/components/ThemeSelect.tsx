import { useEffect, useState } from 'react';
type Theme = 'system' | 'light' | 'dark';
const key = 'monitor-theme';
function saved(): Theme { try { const v=localStorage.getItem(key); return v==='dark'||v==='light'||v==='system'?v:'dark'; } catch { return 'dark'; } }
export function ThemeSelect() {
  const [theme,setTheme]=useState<Theme>(saved);
  useEffect(()=>{
    const media=window.matchMedia('(prefers-color-scheme: dark)');
    const apply=()=>{const dark=theme==='dark'||(theme==='system'&&media.matches);document.documentElement.dataset.theme=dark?'dark':'light';document.querySelector('meta[name="theme-color"]')?.setAttribute('content',dark?'#111b19':'#f5f7f6');};
    apply();media.addEventListener('change',apply);
    const sync=(e:StorageEvent)=>{if(e.key===key||e.key===null)setTheme(saved());};window.addEventListener('storage',sync);
    return()=>{media.removeEventListener('change',apply);window.removeEventListener('storage',sync);};
  },[theme]);
  return <label className="theme-select">Tema <select aria-label="Tema del dashboard" value={theme} onChange={e=>{const value=e.target.value as Theme;setTheme(value);try{localStorage.setItem(key,value);}catch{/* Preference stays usable for this visit. */}}}><option value="system">Sistema</option><option value="light">Claro</option><option value="dark">Oscuro</option></select></label>;
}
