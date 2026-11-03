import { Fragment, useEffect, useMemo, useState } from 'react';
import { InvestigationsPanel } from '../components/InvestigationsPanel';
import { AppShell } from '../components/AppShell';
import { apiClient } from '../api/client';
import type { SecurityEvent } from '../types';

function severityClass(severity: SecurityEvent['severity']): string {
  switch (severity) {
    case 'CRITICAL':
      return 'border-red-500/40 bg-red-500/10 text-red-700';
    case 'HIGH':
      return 'border-orange-500/40 bg-orange-500/10 text-orange-700';
    case 'MEDIUM':
      return 'border-yellow-500/40 bg-yellow-500/10 text-yellow-700';
    default:
      return 'border-emerald-500/40 bg-emerald-500/10 text-emerald-700';
  }
}

export function SecurityPage() {
  const [error, setError] = useState('');
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [severityFilter, setSeverityFilter] = useState<string>('ALL');
  const [mitreFilter, setMitreFilter] = useState('');
  const [fromDate, setFromDate] = useState('');
  const [toDate, setToDate]     = useState('');
  const [expandedId, setExpandedId] = useState<number | null>(null);

  const load = async () => {
    setIsLoading(true);
    setError('');
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
      setError('No se pudieron consultar los eventos. Los registros anteriores pueden estar desactualizados.');
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
          className="rounded-lg border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-17715a)]/10 px-4 py-2 text-sm font-medium text-[var(--palette-17715a)] hover:border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-17715a)]/20"
        >
          Actualizar
        </button>
      }
    >
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      <p className="muted">Resumen de los eventos consultados (hasta 500). Fechas de filtro en UTC.</p>
      <InvestigationsPanel />
      {/* Summary cards */}
      <section className="grid gap-3 md:grid-cols-4">
        <article className="rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] p-4">
          <p className="text-xs text-[var(--palette-53685c)]">Eventos de hoy (UTC)</p>
          <p className="mt-1 text-2xl font-semibold text-[var(--palette-53685c)]">{error || isLoading ? '—' : totalToday}</p>
        </article>
        <article className="rounded-xl border border-red-500/30 bg-red-500/10 p-4">
          <p className="text-xs text-red-700">Críticas</p>
          <p className="mt-1 text-2xl font-semibold text-red-700">{error || isLoading ? '—' : criticalCount}</p>
        </article>
        <article className="rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] p-4">
          <p className="text-xs text-[var(--palette-53685c)]">Dispositivos afectados</p>
          <p className="mt-1 text-2xl font-semibold text-[var(--palette-53685c)]">{error || isLoading ? '—' : affectedDevices}</p>
        </article>
        <article className="rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] p-4">
          <p className="text-xs text-[var(--palette-53685c)]">Técnica más frecuente</p>
          <p className="mt-1 text-2xl font-semibold text-[var(--palette-53685c)]">{error || isLoading ? '—' : topTechnique}</p>
        </article>
      </section>

      {/* Filters */}
      <section className="mt-4 rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] p-4">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
          <label className="flex flex-col gap-1 text-xs text-[var(--palette-53685c)]">
            Desde
            <input
              type="date"
              value={fromDate}
              onChange={e => setFromDate(e.target.value)}
              className="rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2 text-sm text-[var(--palette-53685c)]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[var(--palette-53685c)]">
            Hasta
            <input
              type="date"
              value={toDate}
              onChange={e => setToDate(e.target.value)}
              className="rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2 text-sm text-[var(--palette-53685c)]"
            />
          </label>
          <label className="flex flex-col gap-1 text-xs text-[var(--palette-53685c)]">
            Severidad
            <select
              value={severityFilter}
              onChange={e => setSeverityFilter(e.target.value)}
              className="rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2 text-sm text-[var(--palette-53685c)]"
            >
              <option value="ALL">Todas</option>
              <option value="LOW">LOW</option>
              <option value="MEDIUM">MEDIUM</option>
              <option value="HIGH">HIGH</option>
              <option value="CRITICAL">CRITICAL</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-xs text-[var(--palette-53685c)]">
            Técnica MITRE
            <input
              value={mitreFilter}
              onChange={e => setMitreFilter(e.target.value)}
              placeholder="Ej: T1053"
              className="rounded-lg border border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] px-3 py-2 text-sm text-[var(--palette-53685c)]"
            />
          </label>
          <div className="flex items-end gap-2">
            <button
              type="button"
              onClick={load}
              className="flex-1 rounded-lg border border-[var(--palette-dfe7e3)]/40 bg-[var(--palette-17715a)]/10 px-4 py-2 text-sm font-medium text-[var(--palette-17715a)] hover:border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-17715a)]/20"
            >
              Buscar
            </button>
            <button
              type="button"
              onClick={() => { setSeverityFilter('ALL'); setMitreFilter(''); setFromDate(''); setToDate(''); }}
              className="flex-1 rounded-lg border border-[var(--palette-dfe7e3)] bg-transparent px-4 py-2 text-sm text-[var(--palette-53685c)] hover:text-[var(--palette-53685c)]"
            >
              Limpiar
            </button>
          </div>
        </div>
      </section>

      {/* Table */}
      <section className="mt-4 overflow-hidden rounded-xl border border-[var(--palette-dfe7e3)] bg-[var(--surface)] shadow-2xl">
        {isLoading ? (
          <div className="px-6 py-10 text-center text-[var(--palette-53685c)]">Cargando eventos de seguridad…</div>
        ) : events.length === 0 ? (
          <div className="px-6 py-10 text-center text-[var(--palette-53685c)]">Sin eventos para los filtros seleccionados.</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Fecha/Hora</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Dispositivo</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Query</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Pack</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">MITRE</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Severidad</th>
                  <th className="px-6 py-3 text-left text-xs font-bold uppercase tracking-wider text-[var(--palette-17715a)]">Detalle</th>
                </tr>
              </thead>
              <tbody>
                {events.map(ev => (
                  <Fragment key={ev.id}>
                    <tr
                      key={ev.id}
                      className="cursor-pointer border-b border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-f5f7f6)]"
                      onClick={() => setExpandedId(expandedId === ev.id ? null : ev.id)}
                    >
                      <td className="px-6 py-3 text-[var(--palette-53685c)]">{new Date(ev.timestamp).toLocaleString()}</td>
                      <td className="px-6 py-3 font-sans text-xs text-[var(--palette-53685c)]">{ev.device_id.slice(0, 13)}…</td>
                      <td className="px-6 py-3 text-[var(--palette-53685c)]">{ev.query_name}</td>
                      <td className="px-6 py-3 text-xs text-[var(--palette-53685c)]">{ev.query_pack ?? '-'}</td>
                      <td className="px-6 py-3">
                        {ev.mitre_technique ? (
                          <a
                            href={`https://attack.mitre.org/techniques/${ev.mitre_technique}`}
                            target="_blank"
                            rel="noreferrer"
                            onClick={e => e.stopPropagation()}
                            className="text-[var(--palette-17715a)] underline decoration-[var(--palette-17715a)]/50 underline-offset-2"
                          >
                            {ev.mitre_technique}
                          </a>
                        ) : (
                          <span className="text-[var(--palette-53685c)]">-</span>
                        )}
                      </td>
                      <td className="px-6 py-3">
                        <span className={`rounded-full border px-2 py-1 text-xs font-semibold ${severityClass(ev.severity)}`}>
                          {ev.severity}
                        </span>
                      </td>
                      <td className="px-6 py-3 text-xs text-[var(--palette-53685c)]">
                        {expandedId === ev.id ? '▲ ocultar' : '▼ ver datos'}
                      </td>
                    </tr>
                    {expandedId === ev.id && (
                      <tr key={`${ev.id}-detail`} className="border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
                        <td colSpan={7} className="px-8 py-4">
                          <pre className="max-h-60 overflow-auto rounded-lg bg-[var(--palette-f5f7f6)] p-4 font-sans text-xs text-[var(--palette-53685c)]">
                            {JSON.stringify(ev.raw_data, null, 2)}
                          </pre>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </AppShell>
  );
}
