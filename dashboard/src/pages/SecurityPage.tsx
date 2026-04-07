import { useEffect, useMemo, useState } from 'react';
import { AppShell } from '../components/AppShell';
import { apiClient } from '../api/client';
import type { SecurityEvent } from '../types';

function severityClass(severity: SecurityEvent['severity']): string {
  switch (severity) {
    case 'CRITICAL':
      return 'border-red-500/40 bg-red-500/10 text-red-300';
    case 'HIGH':
      return 'border-orange-500/40 bg-orange-500/10 text-orange-300';
    case 'MEDIUM':
      return 'border-yellow-500/40 bg-yellow-500/10 text-yellow-300';
    default:
      return 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300';
  }
}

export function SecurityPage() {
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [severityFilter, setSeverityFilter] = useState<string>('ALL');
  const [mitreFilter, setMitreFilter] = useState('');
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate]     = useState('');
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const load = async () => {
    setIsLoading(true);
    try {
      const data = await apiClient.getSecurityEvents({
        severity:    severityFilter !== 'ALL' ? severityFilter : undefined,
        mitreFilter: mitreFilter.trim() || undefined,
        from:        fromDate ? `${fromDate}T00:00:00Z` : undefined,
        to:          toDate   ? `${toDate}T23:59:59Z`   : undefined,
        limit:       500,
      });
      setEvents(data);
    } catch {
      setEvents([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => { load(); }, []); // eslint-disable-line react-hooks/exhaustive-deps

  /* ---- summary counters (client-side from already-filtered data) ---- */
  const totalToday = useMemo(() => {
    const today = new Date().toISOString().slice(0, 10);
    return events.filter(e => e.timestamp.startsWith(today)).length;
  }, [events]);

  const criticalCount  = useMemo(() => events.filter(e => e.severity === 'CRITICAL').length, [events]);
  const affectedDevices = useMemo(() => new Set(events.map(e => e.device_id)).size, [events]);

  const topTechnique = useMemo(() => {
    const counter = new Map<string, number>();
    for (const e of events) {
      const t = e.mitre_technique || 'N/A';
      counter.set(t, (counter.get(t) || 0) + 1);
    }
    let best = '-'; let max = 0;
    for (const [t, n] of counter) { if (n > max) { best = t; max = n; } }
    return best;
  }, [events]);

  return (
    <AppShell
      currentPage="security"
      title="Seguridad"
      subtitle="Eventos de seguridad — osquery + MITRE ATT&CK"
      actions={
        <button
          onClick={load}
          className="rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
        >
          Actualizar
        </button>
      }
    >
      {/* Summary cards */}
      <section className="grid gap-3 md:grid-cols-4">
        <article className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-4">
          <p className="text-xs text-[#8ea0cf]">Alertas hoy</p>
          <p className="mt-1 text-2xl font-semibold text-[#e4e6eb]">{totalToday}</p>
        </article>
        <article className="rounded-xl border border-red-500/30 bg-red-500/10 p-4">
          <p className="text-xs text-red-200">Críticas</p>
          <p className="mt-1 text-2xl font-semibold text-red-300">{criticalCount}</p>
        </article>
        <article className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-4">
          <p className="text-xs text-[#8ea0cf]">Dispositivos afectados</p>
          <p className="mt-1 text-2xl font-semibold text-[#e4e6eb]">{affectedDevices}</p>
        </article>
        <article className="rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-4">
          <p className="text-xs text-[#8ea0cf]">Técnica más frecuente</p>
          <p className="mt-1 text-2xl font-semibold text-[#e4e6eb]">{topTechnique}</p>
        </article>
      </section>

      {/* Filters */}
      <section className="mt-4 rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] p-4">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
          <label className="flex flex-col gap-1 text-xs text-[#8ea0cf]">
            Desde
            <input
              type="date"
              value={fromDate}
              onChange={e => setFromDate(e.target.value)}
              className="rounded-lg border border-[#273153] bg-[#0b1230] px-3 py-2 text-sm text-[#e4e6eb]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[#8ea0cf]">
            Hasta
            <input
              type="date"
              value={toDate}
              onChange={e => setToDate(e.target.value)}
              className="rounded-lg border border-[#273153] bg-[#0b1230] px-3 py-2 text-sm text-[#e4e6eb]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[#8ea0cf]">
            Severidad
            <select
              value={severityFilter}
              onChange={e => setSeverityFilter(e.target.value)}
              className="rounded-lg border border-[#273153] bg-[#0b1230] px-3 py-2 text-sm text-[#e4e6eb]"
            >
              <option value="ALL">Todas</option>
              <option value="LOW">LOW</option>
              <option value="MEDIUM">MEDIUM</option>
              <option value="HIGH">HIGH</option>
              <option value="CRITICAL">CRITICAL</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs text-[#8ea0cf]">
            Técnica MITRE
            <input
              value={mitreFilter}
              onChange={e => setMitreFilter(e.target.value)}
              placeholder="Ej: T1053"
              className="rounded-lg border border-[#273153] bg-[#0b1230] px-3 py-2 text-sm text-[#e4e6eb]"
            />
          </label>
          <div className="flex items-end gap-2">
            <button
              type="button"
              onClick={load}
              className="flex-1 rounded-lg border border-[#00d9ff]/40 bg-[#00d9ff]/10 px-4 py-2 text-sm font-medium text-[#00d9ff] hover:border-[#00d9ff] hover:bg-[#00d9ff]/20"
            >
              Buscar
            </button>
            <button
              type="button"
              onClick={() => { setSeverityFilter('ALL'); setMitreFilter(''); setFromDate(''); setToDate(''); }}
              className="flex-1 rounded-lg border border-[#273153] bg-transparent px-4 py-2 text-sm text-[#8ea0cf] hover:text-[#e4e6eb]"
            >
              Limpiar
            </button>
          </div>
        </div>
      </section>

      {/* Table */}
      <section className="mt-4 overflow-hidden rounded-xl border border-[#1e2339] bg-gradient-to-br from-[#131829] to-[#0a0e27] shadow-2xl">
        {isLoading ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Cargando eventos de seguridad…</div>
        ) : events.length === 0 ? (
          <div className="px-6 py-10 text-center text-[#a0a5b2]">Sin eventos para los filtros seleccionados.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Fecha/Hora</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Dispositivo</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Query</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Pack</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">MITRE</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Severidad</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[#00d9ff]">Detalle</th>
                </tr>
              </thead>
              <tbody>
                {events.map(ev => (
                  <>
                    <tr
                      key={ev.id}
                      className="cursor-pointer border-b border-[#1e2339] hover:bg-[#131829]"
                      onClick={() => setExpandedId(expandedId === ev.id ? null : ev.id)}
                    >
                      <td className="px-6 py-3 text-[#a0a5b2]">{new Date(ev.timestamp).toLocaleString()}</td>
                      <td className="px-6 py-3 font-mono text-xs text-[#8ea0cf]">{ev.device_id.slice(0, 13)}…</td>
                      <td className="px-6 py-3 text-[#e4e6eb]">{ev.query_name}</td>
                      <td className="px-6 py-3 text-xs text-[#717579]">{ev.query_pack ?? '-'}</td>
                      <td className="px-6 py-3">
                        {ev.mitre_technique ? (
                          <a
                            href={`https://attack.mitre.org/techniques/${ev.mitre_technique}`}
                            target="_blank"
                            rel="noreferrer"
                            onClick={e => e.stopPropagation()}
                            className="text-[#00d9ff] underline decoration-[#00d9ff]/50 underline-offset-2"
                          >
                            {ev.mitre_technique}
                          </a>
                        ) : (
                          <span className="text-[#717579]">-</span>
                        )}
                      </td>
                      <td className="px-6 py-3">
                        <span className={`rounded-full border px-2 py-1 text-xs font-semibold ${severityClass(ev.severity)}`}>
                          {ev.severity}
                        </span>
                      </td>
                      <td className="px-6 py-3 text-xs text-[#6a7391]">
                        {expandedId === ev.id ? '▲ ocultar' : '▼ ver datos'}
                      </td>
                    </tr>
                    {expandedId === ev.id && (
                      <tr key={`${ev.id}-detail`} className="border-b border-[#1e2339] bg-[#0b0f26]">
                        <td colSpan={7} className="px-8 py-4">
                          <pre className="max-h-60 overflow-auto rounded-lg bg-[#060a1a] p-4 font-mono text-xs text-[#8ea0cf]">
                            {JSON.stringify(ev.raw_data, null, 2)}
                          </pre>
                        </td>
                      </tr>
                    )}
                  </>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </AppShell>
  );
}
