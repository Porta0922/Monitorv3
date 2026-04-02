import { useOverview } from '../hooks/useOverview';

export function OverviewCard() {
  const { data, isLoading, error } = useOverview();

  if (isLoading) {
    return (
      <div className="bg-[#131829] rounded-xl border border-[#1e2339] p-8">
        <p className="text-[#a0a5b2]">Cargando resumen del día...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-500/10 rounded-xl border border-red-500/30 p-8">
        <p className="text-red-400">Error: {error}</p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="bg-[#131829] rounded-xl border border-[#1e2339] p-8">
        <p className="text-[#a0a5b2]">Sin datos disponibles</p>
      </div>
    );
  }

  return (
    <div className="bg-gradient-to-br from-[#131829] to-[#0a0e27] rounded-xl border border-[#1e2339] overflow-hidden shadow-2xl">
      <div className="px-6 py-4 border-b border-[#1e2339] bg-[#0a0e27]">
        <h2 className="text-xl font-bold text-[#e4e6eb]">📊 Resumen del Día</h2>
      </div>

      <div className="p-6">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
          {/* Devices Today */}
          <div className="bg-[#0a0e27] rounded-lg p-4 border border-[#1e2339] hover:border-[#00d9ff]/50 transition-colors">
            <p className="text-[#a0a5b2] text-xs font-semibold uppercase tracking-wide mb-2">Dispositivos</p>
            <p className="text-3xl font-bold text-[#00d9ff]">{data.devices_today}</p>
            <p className="text-xs text-[#717579] mt-2">activos hoy</p>
          </div>

          {/* Active Time */}
          <div className="bg-[#0a0e27] rounded-lg p-4 border border-[#1e2339] hover:border-[#00ff88]/50 transition-colors">
            <p className="text-[#a0a5b2] text-xs font-semibold uppercase tracking-wide mb-2">Activo</p>
            <p className="text-3xl font-bold text-[#00ff88]">{Math.round(data.active_time / 3600)}h</p>
            <p className="text-xs text-[#717579] mt-2">{data.active_time}s</p>
          </div>

          {/* Idle Time */}
          <div className="bg-[#0a0e27] rounded-lg p-4 border border-[#1e2339] hover:border-[#00d9ff]/50 transition-colors">
            <p className="text-[#a0a5b2] text-xs font-semibold uppercase tracking-wide mb-2">Inactivo</p>
            <p className="text-3xl font-bold text-[#a0a5b2]">{Math.round(data.idle_time / 3600)}h</p>
            <p className="text-xs text-[#717579] mt-2">{data.idle_time}s</p>
          </div>

          {/* Idle % */}
          <div className="bg-[#0a0e27] rounded-lg p-4 border border-[#1e2339] hover:border-[#00ff88]/50 transition-colors">
            <p className="text-[#a0a5b2] text-xs font-semibold uppercase tracking-wide mb-2">Inactiv %</p>
            <p className="text-3xl font-bold text-[#00ff88]">{data.idle_pct}</p>
            <p className="text-xs text-[#717579] mt-2">del tiempo</p>
          </div>

          {/* Keys Today */}
          <div className="bg-[#0a0e27] rounded-lg p-4 border border-[#1e2339] hover:border-[#00d9ff]/50 transition-colors">
            <p className="text-[#a0a5b2] text-xs font-semibold uppercase tracking-wide mb-2">Teclas</p>
            <p className="text-3xl font-bold text-[#00d9ff]">{data.keys_today}</p>
            <p className="text-xs text-[#717579] mt-2">pulsaciones</p>
          </div>
        </div>
      </div>
    </div>
  );
}
