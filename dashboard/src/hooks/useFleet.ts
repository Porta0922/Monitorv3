import { useEffect, useState } from 'react';
import { apiClient } from '../api/client';
import type { EquipmentPage } from '../api/client';
export function useFleet(query:string) {
  const [equipment,setEquipment]=useState<{key:string;data:EquipmentPage}|null>(null);
  const [errors,setErrors]=useState<string[]>([]);
  const [updatedAt,setUpdatedAt]=useState<Date|null>(null);
  const [refreshing,setRefreshing]=useState(true);
  const [revision,setRevision]=useState(0);
  useEffect(()=>{
    const abort=new AbortController();let busy=false;
    const load=async()=>{
      if(busy)return;busy=true;setRefreshing(true);
      const results=await Promise.allSettled([apiClient.getEquipmentPage(query,abort.signal)] as const);
      if(abort.signal.aborted)return;
      const failures:string[]=[];const [page]=results;
      if(page.status==='fulfilled'){setEquipment({key:query,data:page.value});setUpdatedAt(new Date());}else failures.push('equipos');
      setErrors(failures);setRefreshing(false);busy=false;
    };
    const debounce=setTimeout(()=>void load(),250);
    const timer=setInterval(()=>{if(!document.hidden)void load();},30000);
    return ()=>{abort.abort();clearTimeout(debounce);clearInterval(timer);};
  },[query,revision]);
  const page=equipment?.key===query?equipment.data:null;
  return {devices:page?.devices??null,total:page?.total??0,counts:page?.counts??null,groups:page?.groups??equipment?.data.groups??[],errors,updatedAt,refreshing,refresh:()=>setRevision(v=>v+1)};
}
