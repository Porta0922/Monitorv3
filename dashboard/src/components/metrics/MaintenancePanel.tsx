import { StoragePanel } from './StoragePanel';
import { DeadLettersPanel } from './DeadLettersPanel';
import { useEffect, useState } from 'react';
import { apiClient } from '../../api/client';
import type { MaintenanceStatus } from '../../types';

const date = (value: string | null) => value ? new Date(value).toLocaleString() : 'Sin ejecución registrada';
const names: Record<string, string> = { deduplication: 'Deduplicación', security: 'Eventos de seguridad', 'inventory-changes': 'Cambios de software', activity: 'Actividad', usb: 'USB', wifi: 'WiFi', audit: 'Auditoría', 'resolved-alerts': 'Alertas resueltas' };

export function MaintenancePanel() {
  const [data, setData] = useState<MaintenanceStatus | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    const refresh = async () => {
      try {
        const next = await apiClient.getMaintenanceStatus(controller.signal);
        if (!controller.signal.aborted) { setData(next); setError(false); }
      } catch {
        if (!controller.signal.aborted) setError(true);
      }
    };
    void refresh();
    const timer = setInterval(() => void refresh(), 60000);
    return () => { controller.abort(); clearInterval(timer); };
  }, []);
  return <><DeadLettersPanel /><StoragePanel /><details className="shrink-0 rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] p-4 text-sm text-[var(--palette-53685c)]">
    <summary className="cursor-pointer">Mantenimiento de históricos{error ? ' · No disponible' : ''}</summary>
    {error ? <p role="alert" className="mt-3 text-red-700">No se pudo consultar el estado. Se reintentará en un minuto.</p>
      : !data ? <p className="mt-3">Cargando estado…</p> : <div className="mt-3 space-y-3">
        <p>Base completa: {(data.database_size_bytes / 1024 / 1024).toFixed(1)} MiB · Consulta: {date(data.observed_at)}</p>
        {!!data.relations?.length && <details><summary>Tamaño por tabla y relación física</summary>
          <p>Hasta {data.relation_limit ?? 200} relaciones en orden alfabético. Incluye relaciones internas y chunks de Timescale; no es un total agregado por hypertable. Filas estimadas por PostgreSQL.{data.relations_truncated ? ' Listado parcial: hay más relaciones.' : ''}</p>
          <div className="overflow-x-auto"><table className="w-full text-left text-xs"><thead><tr><th>Relación</th><th>Total</th><th>Índices</th><th>Filas estimadas</th></tr></thead><tbody>{data.relations.map(r=><tr key={`${r.schema_name}.${r.relation_name}`}><td>{r.schema_name}.{r.relation_name}</td><td>{(r.total_bytes/1048576).toFixed(1)} MiB</td><td>{(r.index_bytes/1048576).toFixed(1)} MiB</td><td>{r.estimated_rows ?? 'Sin estimación'}</td></tr>)}</tbody></table></div>
        </details>}
        {data.worker_settings && <details><summary>Capacidad de procesos de mantenimiento</summary><dl>{Object.entries(data.worker_settings).map(([name,value])=><div key={name}><dt>{name}</dt><dd>{value}</dd></div>)}</dl><p>Configuración observada del servidor. Un fallo de job requiere revisar también sus registros y la disponibilidad de procesos; estos valores no confirman que haya ejecutado correctamente.</p></details>}
        <p>Última compactación de métricas: {date(data.metrics?.last_success_at ?? null)}</p>
        <p>Historial de ejecuciones; no confirma que la limpieza siga habilitada. Pendientes medidos al finalizar cada lote.</p>
        {data.history.length === 0 ? <p>Sin ejecuciones de limpieza histórica registradas.</p> :
          <div className="overflow-x-auto"><table className="w-full text-left text-xs">
            <thead><tr>{['Tarea', 'Retención', 'Último éxito', 'Eliminados', 'Pendientes', 'Estado'].map(h => <th className="p-2" key={h}>{h}</th>)}</tr></thead>
            <tbody>{data.history.map(job => <tr key={job.job}>
              <td className="p-2">{names[job.job] ?? job.job}</td><td className="p-2">{job.retention_days} días</td>
              <td className="p-2">{date(job.last_success_at)}</td><td className="p-2">{job.last_rows_processed}</td>
              <td className="p-2">{job.pending_rows_lower_bound === null ? 'Desconocido' : `${job.backlog_truncated ? '≥ ' : ''}${job.pending_rows_lower_bound}`}</td>
              <td className="p-2">{job.last_error ? <span className="text-red-700">Falló: {job.last_error}</span> : 'Último lote correcto'}</td>
            </tr>)}</tbody>
          </table></div>}
      </div>}
  </details></>;
}
