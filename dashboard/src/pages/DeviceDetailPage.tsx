import {useRemoteSection} from '../hooks/useRemoteSection';
import {CollectionTable} from '../components/CollectionTable';
import {SessionActivityPanel} from '../components/metrics/SessionActivityPanel';
import {AgentCredentialsPanel} from '../components/AgentCredentialsPanel';
import {DeviceOverview} from '../components/DeviceOverview';
import {DeviceActivityExplorer} from '../components/DeviceActivityExplorer';
import type {HourGroup} from '../api/desktop';
import React, { useCallback, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { apiClient } from '../api/client';
import { AppShell } from '../components/AppShell';
import { InventoryStatePanel } from '../components/InventoryStatePanel';
import { DeviceTelemetryPanel } from '../components/metrics/DeviceTelemetryPanel';
import { DailyMetricsPanel } from '../components/metrics/DailyMetricsPanel';
import type { USBEvent, WifiEvent, NodeResourceMetric } from '../types';

type TabKey = 'overview' | 'activity' | 'inventory' | 'usb' | 'wifi' | 'resources' | 'diagnostic';
type WifiStateFilter = 'all' | 'connected' | 'disconnected';

interface UsbSummaryRow {
  key: string;
  device_key: string;
  timestamp: string;
  device_name: string;
  serial_number?: string;
  hardware_id: string;
  action: 'IN' | 'OUT';
  pair_id: string;
  pair_step: number;
  pair_started_at: number;
}

interface WifiEventWithDuration extends WifiEvent {
  connected_duration_seconds: number | null;
}

export function DeviceDetailPage() {
  const { deviceId } = useParams<{ deviceId: string }>();
  const navigate = useNavigate();

  const [params, setParams] = useSearchParams();
  const change = (key: string, value: string) => setParams(previous => { const next = new URLSearchParams(previous); next.set(key,value); return next; }, {replace:true});
  const tabs = ['overview','activity','inventory','usb','wifi','resources','diagnostic'];
  const tab = (tabs.includes(params.get('tab') ?? '') ? params.get('tab') : 'overview') as TabKey;
  const setTab = (value: TabKey) => change('tab', value);
  const dateToday = new Date().toISOString().slice(0,10);
  const requestedDate = params.get('date') ?? dateToday;
  const selectedDate = /^\d{4}-\d{2}-\d{2}$/.test(requestedDate) && Number.isFinite(Date.parse(requestedDate)) && new Date(requestedDate).toISOString().slice(0,10) === requestedDate && requestedDate <= dateToday ? requestedDate : dateToday;
  const setSelectedDate = (value: string) => change('date', value);
  const wifiStateFilter = (['connected','disconnected'].includes(params.get('wifi') ?? '') ? params.get('wifi') : 'all') as WifiStateFilter;
  const setWifiStateFilter = (value: WifiStateFilter) => change('wifi', value);
  const [isExporting, setIsExporting] = useState(false);
  const [exportError, setExportError] = useState('');
  const equipment = useRemoteSection(useCallback(async () => {
    const devices = await apiClient.getDevices();
    const selected = devices.find(d => d.device_id === deviceId);
    if (!selected) throw new Error('Equipo no disponible');
    return selected;
  }, [deviceId]));
  const apps = useRemoteSection(useCallback(() => apiClient.getApps(deviceId!), [deviceId]));
  const openApps = useRemoteSection(useCallback(() => apiClient.getRunningApps(deviceId!), [deviceId]));
  const usb = useRemoteSection(useCallback(() => apiClient.getUsbHistory(deviceId!,250,selectedDate), [deviceId,selectedDate]));
  const wifi = useRemoteSection(useCallback(() => apiClient.getWifiHistory(deviceId!,250,selectedDate), [deviceId,selectedDate]));
  const resources = useRemoteSection(useCallback(() => apiClient.getDeviceResources(deviceId!,selectedDate,2880), [deviceId,selectedDate]));
  const activity = useRemoteSection(useCallback(() => apiClient.getHistoryHourlyPrograms(deviceId!,selectedDate), [deviceId,selectedDate]));
  const device = equipment.data;
  const usbEvents = useMemo(() => usb.data ?? [], [usb.data]);
  const wifiEvents = useMemo(() => wifi.data ?? [], [wifi.data]);
  const resourceMetrics = useMemo(() => resources.data ?? [], [resources.data]);
  const hourlyPrograms = activity.data ?? [];
  const isLoading = equipment.loading;
  const error = equipment.error;
  const refresh = () => {equipment.refresh(); apps.refresh(); openApps.refresh(); usb.refresh(); wifi.refresh(); resources.refresh(); activity.refresh();};
  const returnQuery = params.get('equipmentQuery') ?? '';

  const formatTopProcessCpu = (value?: number) => {
    return Math.round(Math.min(100, Math.max(0, value || 0)));
  };

  const formatTopProcessMemoryMb = (value?: number) => {
    const raw = Math.max(0, value || 0);
    // Backward compatibility for legacy rows persisted as KB-as-MB.
    const normalized = raw > 8192 ? raw / 1024 : raw;
    return Math.round(normalized);
  };

  const shortAppName = (rawName: string) => {
    const cleaned = (rawName || '').trim();
    const normalized = cleaned.toLowerCase();
    if (!cleaned || normalized === 'unknown' || normalized === 'n/a' || normalized === '<unknown>' || normalized === '(unknown)') {
      return 'Sin identificar';
    }

    const normalizedPath = cleaned.replace(/\\/g, '/');
    const lastSegment = normalizedPath.split('/').pop() || cleaned;
    const aliasMap: Record<string, string> = {
      'chrome.exe': 'Google Chrome',
      'msedge.exe': 'Microsoft Edge',
      'firefox.exe': 'Mozilla Firefox',
      'code.exe': 'Visual Studio Code',
    };
    const mapped = aliasMap[lastSegment.toLowerCase()];
    if (mapped) {
      return mapped;
    }
    return lastSegment.length > 38 ? `${lastSegment.slice(0, 35)}...` : lastSegment;
  };

  const tabClass = (key: TabKey) =>
    `rounded-full border px-4 py-2 text-xs font-semibold tracking-wide transition-all ${
      tab === key
        ? 'border-[var(--palette-dfe7e3)] bg-[var(--palette-17715a)]/15 text-[var(--palette-17715a)]'
        : 'border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] text-[var(--palette-53685c)] hover:border-[var(--palette-dfe7e3)]/50 hover:text-[var(--palette-53685c)]'
    }`;

  const resourcePeak = useMemo(() => {
    if (resourceMetrics.length === 0) {
      return null;
    }

    return resourceMetrics.reduce((best: NodeResourceMetric | null, current: NodeResourceMetric) => {
      if (!best) return current;
      const bestScore = (best.cpu_percent || 0) + (best.memory_percent || 0);
      const currentScore = (current.cpu_percent || 0) + (current.memory_percent || 0);
      return currentScore > bestScore ? current : best;
    }, resourceMetrics[0] || null);
  }, [resourceMetrics]);

  const usbSummaryRows = useMemo<UsbSummaryRow[]>(() => {
    const normalized = (value?: string) => (value || '').trim();
    const isUnknownLike = (value?: string) => {
      const v = normalized(value).toLowerCase();
      return !v || v === 'unknown' || v === 'n/a' || v === '<unknown>' || v === '(unknown)';
    };

    const eventsByDevice = new Map<string, USBEvent[]>();

    for (const event of usbEvents) {
      const serial = isUnknownLike(event.serial_number) ? '' : normalized(event.serial_number);
      const hardware = isUnknownLike(event.hardware_id) ? '' : normalized(event.hardware_id);
      const name = normalized(event.device_name);

      const uniqueKey = serial || hardware || name;

      if (!uniqueKey) {
        continue;
      }

      const list = eventsByDevice.get(uniqueKey) || [];
      list.push(event);
      eventsByDevice.set(uniqueKey, list);
    }

    const pairedRows: UsbSummaryRow[] = [];

    for (const [deviceKey, events] of eventsByDevice.entries()) {
      const sortedAsc = [...events].sort(
        (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
      );

      let pairCounter = 0;
      let hasOpenConnection = false;
      let pairStartedAt = 0;

      for (const event of sortedAsc) {
        if (event.action === 'IN' || !hasOpenConnection) {
          pairCounter += 1;
          pairStartedAt = new Date(event.timestamp).getTime();
        }

        const pairId = `${deviceKey}#${pairCounter}`;

        pairedRows.push({
          key: `${pairId}|${event.action}|${event.timestamp}`,
          device_key: deviceKey,
          timestamp: event.timestamp,
          device_name: event.device_name,
          serial_number: event.serial_number,
          hardware_id: event.hardware_id,
          action: event.action,
          pair_id: pairId,
          pair_step: event.action === 'IN' ? 1 : 2,
          pair_started_at: pairStartedAt,
        });

        if (event.action === 'IN') {
          hasOpenConnection = true;
        }
        if (event.action === 'OUT') {
          hasOpenConnection = false;
        }
      }
    }

    return pairedRows.sort((a, b) => {
      const pairCompare = b.pair_started_at - a.pair_started_at;
      if (pairCompare !== 0) return pairCompare;
      const stepCompare = a.pair_step - b.pair_step;
      if (stepCompare !== 0) return stepCompare;
      return new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
    });
  }, [usbEvents]);

  const wifiEventsWithDuration = useMemo<WifiEventWithDuration[]>(() => {
    const chronological=[...wifiEvents].sort((a,b)=>Date.parse(a.timestamp)-Date.parse(b.timestamp));
    return chronological.map((event,index)=>{
      const next=chronological.slice(index+1).find(row=>row.interface_name===event.interface_name);
      const seconds=event.state==='connected'&&next ? Math.max(0,Math.floor((Date.parse(next.timestamp)-Date.parse(event.timestamp))/1000)) : null;
      return {...event,connected_duration_seconds:seconds};
    }).reverse();
  }, [wifiEvents]);

  const filteredWifiEvents = useMemo(() => {
    return wifiEventsWithDuration.filter((event: WifiEventWithDuration) => {
      const stateMatches =
        wifiStateFilter === 'all' ? true : event.state.toLowerCase() === wifiStateFilter;

      const eventDate = new Date(event.timestamp).toISOString().slice(0, 10);
      const dateMatches = eventDate === selectedDate;

      return stateMatches && dateMatches;
    });
  }, [wifiEventsWithDuration, wifiStateFilter, selectedDate]);

  const formatLongDuration = (seconds?: number) => {
    const safeSeconds = Math.max(0, seconds || 0);
    const hours = Math.floor(safeSeconds / 3600);
    const minutes = Math.floor((safeSeconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  const handleExportCsv = async () => {
    if (!deviceId) return;
    setIsExporting(true);
    setExportError('');
    try {
      const blob = await apiClient.exportCsv({
        deviceId,
        from: selectedDate,
        to: selectedDate,
      });

      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ame_${deviceId.slice(0, 8)}_${selectedDate}.csv`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.URL.revokeObjectURL(url);
    } catch {
      setExportError('No fue posible exportar la actividad. Comprueba tu conexión y tus permisos, y vuelve a intentarlo.');
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <AppShell
      currentPage="dashboard"
      contentClassName="device-detail"
      title={device ? device.nickname || device.hostname : 'Detalle del equipo'}
      subtitle="Recursos, actividad y conexiones del equipo seleccionado."
      actions={
        <button
          onClick={() => navigate(`/dashboard${returnQuery ? `?${returnQuery}` : ''}`)}
          className="rounded-full border border-[var(--palette-dfe7e3)]/50 bg-[var(--palette-17715a)]/10 px-4 py-2 text-xs font-semibold tracking-wide text-[var(--palette-17715a)] hover:border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-17715a)]/20"
        >
          Volver a los equipos
        </button>
      }
    >
      {device && <div className="device-identity"><span className={device.online?'device-online':'device-offline'}>{device.online?'Conectado':'Sin conexión'}</span><strong>{device.hostname}</strong><span className="muted">Última señal: {new Date(device.last_seen).toLocaleString()}</span><details><summary>Identificación</summary><p>Identificador: {device.device_id}</p><p>MAC: {device.mac_address||'Sin informar'}</p></details><button className="button button-small" disabled={isLoading} onClick={refresh}>{isLoading?'Actualizando…':'Actualizar'}</button></div>}

      <section className="device-navigation">
        <nav className="device-tabs" aria-label="Secciones del equipo">{([['overview','Resumen'],['activity','Actividad'],['inventory','Aplicaciones'],['resources','Recursos'],['usb','USB'],['wifi','WiFi'],['diagnostic','Diagnóstico']] as [TabKey,string][]).map(([key,label])=><button key={key} className={tabClass(key)} aria-current={tab===key?'page':undefined} onClick={()=>setTab(key)}>{label}</button>)}</nav>

        {(tab === 'overview' || tab === 'activity' || tab === 'resources' || tab === 'usb' || tab === 'wifi') && (
          <div className="flex items-center gap-2">
            <label htmlFor="device-date" className="muted">Fecha de consulta (UTC)</label>
            <input
              type="date" id="device-date"
              value={selectedDate}
              max={dateToday}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSelectedDate(e.target.value || dateToday)}
              className="rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-1.5 font-sans text-[12px] text-[var(--palette-53685c)] cursor-pointer"
            />
            <button
              onClick={handleExportCsv}
              disabled={isExporting || apiClient.getRole()==='viewer'} title={apiClient.getRole()==='viewer'?'La exportación requiere un rol operador o administrador':undefined}
              className="rounded-full border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-31764d)]/10 px-3 py-1.5 font-sans text-[12px] text-[var(--palette-31764d)] hover:border-[var(--palette-dfe7e3)] disabled:opacity-60"
            >
              {isExporting ? 'Exportando…' : 'Exportar actividad'}
            </button>
          </div>
        )}

        {tab === 'wifi' && (
          <div className="flex items-center gap-2">
            <span className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Estado</span>
            <select
              aria-label="Estado de conexión WiFi" value={wifiStateFilter}
              onChange={(event: React.ChangeEvent<HTMLSelectElement>) => setWifiStateFilter(event.target.value as WifiStateFilter)}
              className="rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-1.5 text-[12px] text-[var(--palette-53685c)]"
            >
              <option value="all">Todos</option>
              <option value="connected">Conectado</option>
              <option value="disconnected">Desconectado</option>
            </select>
          </div>
        )}
      </section>

      {([['overview','Actividad del día',activity],['activity','Actividad del día',activity],['resources','Historial de recursos',resources]] as const).map(([section,label,state]) => tab===section && <div key={section}>
        {state.loading && <p role="status">Consultando {label.toLowerCase()}…</p>}
        {state.error && <p className="notice notice-warning" role="alert">No se pudo actualizar {label.toLowerCase()}. {state.data ? 'Se conserva la consulta anterior de esta fecha.' : 'No hay datos confirmados para esta consulta.'} <button className="button" disabled={state.loading} onClick={state.refresh}>Reintentar {label.toLowerCase()}</button></p>}
      </div>)}

      {tab === 'resources' && resourceMetrics.length > 0 && (
        <section className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-4 py-2.5">
          <span className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">Recursos: {selectedDate}</span>
          <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)]/30 bg-[var(--palette-b2433d)]/10 px-2.5 py-1">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--palette-b2433d)]" />
            <span className="font-sans text-[12px] text-[var(--palette-53685c)]">CPU pico {Math.min(100, Math.round(Math.max(...resourceMetrics.map(metric => metric.cpu_percent))))}%</span>
          </span>
          <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)]/30 bg-[var(--palette-17715a)]/10 px-2.5 py-1">
            <span className="inline-block h-1.5 w-1.5 rounded-full bg-[var(--palette-17715a)]" />
            <span className="font-sans text-[12px] text-[var(--palette-53685c)]">RAM pico {Math.min(100, Math.round(Math.max(...resourceMetrics.map(metric => metric.memory_percent))))}%</span>
          </span>
          {resourcePeak?.top_process_name && (
            <span className="flex items-center gap-1.5 rounded-full border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-2.5 py-1">
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">▸</span>
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">{resourcePeak.top_process_name}</span>
              <span className="font-sans text-[12px] text-[var(--palette-53685c)]">
                {formatTopProcessCpu(resourcePeak.top_process_cpu_percent)}% cpu · {formatTopProcessMemoryMb(resourcePeak.top_process_memory_mb)} MB
              </span>
            </span>
          )}
          <span className="ml-auto font-sans text-[12px] text-[var(--palette-53685c)]">{resourceMetrics.length} muestras</span>
        </section>
      )}

      {deviceId && tab === 'inventory' && <InventoryStatePanel key={deviceId} deviceId={deviceId} />}
      {deviceId && tab === 'diagnostic' && <><DeviceTelemetryPanel key={deviceId} deviceId={deviceId} />{apiClient.getRole()==='admin' && <AgentCredentialsPanel key={`credentials-${deviceId}`} deviceId={deviceId}/>}</>}
      {deviceId && tab === 'overview' && !!device && <DeviceOverview key={deviceId} deviceId={deviceId} groups={hourlyPrograms as HourGroup[]} activityKnown={activity.data !== null} onActivity={()=>setTab('activity')} onResources={()=>setTab('resources')} onDiagnostic={()=>setTab('diagnostic')}/>}
      {deviceId && tab === 'overview' && !!device && <SessionActivityPanel key={`${deviceId}-${selectedDate}`} deviceId={deviceId} initialDay={selectedDate} compact/>}
      {deviceId && tab === 'resources' && <DeviceTelemetryPanel key={deviceId} deviceId={deviceId} resourcesOnly />}
      {tab === 'activity' && activity.data !== null && <DeviceActivityExplorer key={`${deviceId}-${selectedDate}`} groups={hourlyPrograms as HourGroup[]}/>}


      {tab === 'resources' && deviceId && (
        <DailyMetricsPanel deviceId={deviceId} endDate={selectedDate} />
      )}

      {error && <section className="notice notice-error" role="alert">No se pudo consultar el equipo. Puede no estar disponible o no tienes acceso. <button className="text-button" onClick={equipment.refresh}>Reintentar</button></section>}

      {isLoading && !device && <p role="status">Consultando equipo…</p>}
      {exportError && <p className="notice notice-error" role="alert">{exportError}</p>}
      {tab === 'inventory' && <>
        <CollectionTable title="Ventanas reportadas" rows={openApps.data} loading={openApps.loading} error={openApps.error} refresh={openApps.refresh}
          note="Último reporte por sesión; no confirma que la ventana siga abierta. Los reportes de más de tres minutos son antiguos."
          searchText={r=>`${r.app_name} ${r.primary_title} ${r.principal} ${r.exe_path}`} columns={[
            {label:'Aplicación',render:r=>shortAppName(r.app_name)}, {label:'Ventanas',render:r=>r.window_count},
            {label:'Título',render:r=>r.primary_title||'Sin título'},
            {label:'Reporte',render:r=><>{r.stale===true?'Antiguo':r.stale===false?'Reciente':'Vigencia sin confirmar'} · {new Date(r.updated_at).toLocaleString()}</>},
            {label:'Persona / sesión',render:r=><>{r.principal||'Sin atribución'} · {r.session_id||'Origen heredado'}</>},
            {label:'Ruta',render:r=>r.exe_path||'Sin informar'}]}/>
        <details className="panel"><summary>Detecciones históricas de aplicaciones</summary><CollectionTable title="Detecciones heredadas" rows={apps.data} loading={apps.loading} error={apps.error} refresh={apps.refresh}
          note="Estas detecciones no confirman instalación actual. Consulta Software por fuente para conocer el último inventario completo."
          searchText={r=>`${r.app_name} ${r.version}`} columns={[{label:'Aplicación',render:r=>r.app_name},{label:'Versión',render:r=>r.version||'Sin informar'},{label:'Verificación',render:r=>r.verified?'Verificada':'Sin verificar'},{label:'Hash',render:r=>r.exe_hash}]}/></details>
      </>}
      {tab === 'usb' && <CollectionTable title="Conexiones USB" rows={usb.data === null ? null : usbSummaryRows} loading={usb.loading} error={usb.error} refresh={usb.refresh}
        note="Hasta 250 eventos de la fecha UTC seleccionada; búsqueda y paginación sobre esos eventos. Los horarios se muestran en tu zona local."
        searchText={r=>`${r.device_name} ${r.serial_number} ${r.hardware_id}`} columns={[
          {label:'Fecha y hora',render:r=>new Date(r.timestamp).toLocaleString()},{label:'Dispositivo',render:r=>r.device_name},
          {label:'Serie',render:r=>r.serial_number||'Sin informar'},{label:'Hardware',render:r=>r.hardware_id||'Sin informar'},
          {label:'Evento',render:r=>r.action==='IN'?'Conexión':'Desconexión'}]}/>}
      {tab === 'wifi' && <CollectionTable title="Conexiones WiFi" rows={wifi.data === null ? null : filteredWifiEvents} loading={wifi.loading} error={wifi.error} refresh={wifi.refresh}
        note="Hasta 250 eventos del día UTC. Son conexiones reportadas, no una confirmación de la red actual. Duración hasta el siguiente evento de la misma interfaz; sin evento posterior no se estima duración. Horarios en tu zona local."
        searchText={r=>`${r.ssid} ${r.bssid} ${r.interface_name}`} columns={[
          {label:'Fecha y hora',render:r=>new Date(r.timestamp).toLocaleString()},{label:'Red',render:r=>r.ssid||'Sin informar'},
          {label:'BSSID',render:r=>r.bssid||'Sin informar'},{label:'Señal',render:r=>r.signal_percent==null?'Sin informar':`${r.signal_percent}%`},
          {label:'Duración reportada',render:r=>r.connected_duration_seconds==null?'Sin cierre observado':formatLongDuration(r.connected_duration_seconds)},
          {label:'Estado',render:r=>r.state==='connected'?'Conectado':r.state==='disconnected'?'Desconectado':'Sin confirmar'},
          {label:'Interfaz',render:r=>r.interface_name||'Sin informar'}]}/>}
    </AppShell>
  );
}
