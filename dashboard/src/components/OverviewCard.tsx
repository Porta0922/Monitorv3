import { useOverview } from '../hooks/useOverview';

export function OverviewCard() {
  const { data, isLoading, error } = useOverview();

  if (isLoading) {
    return (
      <div className="bg-[var(--palette-f5f7f6)] rounded-xl border border-[var(--palette-dfe7e3)] p-8">
        <p className="text-[var(--palette-53685c)]">Cargando resumen del día...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-500/10 rounded-xl border border-red-500/30 p-8">
        <p className="text-red-700">Error: {error}</p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="bg-[var(--palette-f5f7f6)] rounded-xl border border-[var(--palette-dfe7e3)] p-8">
        <p className="text-[var(--palette-53685c)]">Sin datos disponibles</p>
      </div>
    );
  }

  return (
    <div className="bg-[var(--surface)] rounded-xl border border-[var(--palette-dfe7e3)] overflow-hidden shadow-sm">
      <div className="px-6 py-4 border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
        <h2 className="text-xl font-bold text-[var(--palette-53685c)]">📊 Resumen de personas · día UTC</h2>
      </div>

      <div className="p-6">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
          {/* Devices Today */}
          <div className="bg-[var(--palette-f5f7f6)] rounded-lg p-4 border border-[var(--palette-dfe7e3)] hover:border-[var(--palette-dfe7e3)]/50 transition-colors">
            <p className="text-[var(--palette-53685c)] text-xs font-semibold uppercase tracking-wide mb-2">Dispositivos</p>
            <p className="text-3xl font-bold text-[var(--palette-17715a)]">{data.devices_today}</p>
            <p className="text-xs text-[var(--palette-53685c)] mt-2">activos hoy</p>
          </div>

          {/* Active Time */}
          <div className="bg-[var(--palette-f5f7f6)] rounded-lg p-4 border border-[var(--palette-dfe7e3)] hover:border-[var(--palette-dfe7e3)]/50 transition-colors">
            <p className="text-[var(--palette-53685c)] text-xs font-semibold uppercase tracking-wide mb-2">Activo</p>
            <p className="text-3xl font-bold text-[var(--palette-31764d)]">{data.active_time==null?'Sin datos':`${(data.active_time/3600).toFixed(2)} h-persona`}</p>
            <p className="text-xs text-[var(--palette-53685c)] mt-2">{data.active_time==null?'Sin muestras':`${data.active_time} s`}</p>
          </div>

          {/* Idle Time */}
          <div className="bg-[var(--palette-f5f7f6)] rounded-lg p-4 border border-[var(--palette-dfe7e3)] hover:border-[var(--palette-dfe7e3)]/50 transition-colors">
            <p className="text-[var(--palette-53685c)] text-xs font-semibold uppercase tracking-wide mb-2">Inactivo</p>
            <p className="text-3xl font-bold text-[var(--palette-53685c)]">{data.idle_time==null?'Sin datos':`${(data.idle_time/3600).toFixed(2)} h-persona`}</p>
            <p className="text-xs text-[var(--palette-53685c)] mt-2">{data.idle_time==null?'Sin muestras':`${data.idle_time} s`}</p>
          </div>

          {/* Idle % */}
          <div className="bg-[var(--palette-f5f7f6)] rounded-lg p-4 border border-[var(--palette-dfe7e3)] hover:border-[var(--palette-dfe7e3)]/50 transition-colors">
            <p className="text-[var(--palette-53685c)] text-xs font-semibold uppercase tracking-wide mb-2">Inactiv %</p>
            <p className="text-3xl font-bold text-[var(--palette-31764d)]">{data.idle_pct==null?'Sin datos':`${data.idle_pct.toFixed(1)} %`}</p>
            <p className="text-xs text-[var(--palette-53685c)] mt-2">del tiempo</p>
          </div>

          {/* Keys Today */}
          <div className="bg-[var(--palette-f5f7f6)] rounded-lg p-4 border border-[var(--palette-dfe7e3)] hover:border-[var(--palette-dfe7e3)]/50 transition-colors">
            <p className="text-[var(--palette-53685c)] text-xs font-semibold uppercase tracking-wide mb-2">Teclas</p>
            <p className="text-3xl font-bold text-[var(--palette-17715a)]">{data.keys_today??'Sin datos'}</p>
            <p className="text-xs text-[var(--palette-53685c)] mt-2">pulsaciones</p>
          </div>
        </div>
      </div>
    </div>
  );
}
