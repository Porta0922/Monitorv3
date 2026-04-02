import { useTopApps } from '../hooks/useTopApps';

export function TopAppsTable() {
  const { apps, isLoading, error } = useTopApps();

  if (isLoading) {
    return (
      <div className="bg-[#131829] rounded-xl border border-[#1e2339] p-8">
        <p className="text-[#a0a5b2]">Cargando aplicaciones...</p>
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

  return (
    <div className="bg-gradient-to-br from-[#131829] to-[#0a0e27] rounded-xl border border-[#1e2339] overflow-hidden shadow-2xl">
      <div className="px-6 py-4 border-b border-[#1e2339] bg-[#0a0e27]">
        <h2 className="text-xl font-bold text-[#e4e6eb]">🚀 Top 6 Aplicaciones (Últimos 7 Días)</h2>
      </div>

      {apps.length === 0 ? (
        <div className="px-6 py-8">
          <p className="text-[#a0a5b2]">Sin datos de aplicaciones</p>
        </div>
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="bg-[#0a0e27] border-b border-[#1e2339]">
                  <th className="px-6 py-3 text-left text-xs font-bold text-[#00d9ff] uppercase tracking-wider">Rank</th>
                  <th className="px-6 py-3 text-left text-xs font-bold text-[#00d9ff] uppercase tracking-wider">Aplicación</th>
                  <th className="px-6 py-3 text-right text-xs font-bold text-[#00ff88] uppercase tracking-wider">Horas</th>
                  <th className="px-6 py-3 text-right text-xs font-bold text-[#00ff88] uppercase tracking-wider">Segundos</th>
                </tr>
              </thead>
              <tbody>
                {apps.map((app, index) => (
                  <tr
                    key={app.app_name}
                    className="border-b border-[#1e2339] hover:bg-[#131829] transition-colors"
                  >
                    <td className="px-6 py-3">
                      <span className="inline-flex items-center justify-center w-7 h-7 rounded-full bg-[#00d9ff]/10 text-[#00d9ff] font-mono font-bold text-sm">
                        {index + 1}
                      </span>
                    </td>
                    <td className="px-6 py-3">
                      <p className="font-mono text-[#e4e6eb]">{app.app_name}</p>
                    </td>
                    <td className="px-6 py-3 text-right">
                      <p className="font-mono text-[#00ff88] font-semibold">
                        {(+app.total_duration_hours).toFixed(2)}h
                      </p>
                    </td>
                    <td className="px-6 py-3 text-right">
                      <p className="font-mono text-[#a0a5b2] text-sm">
                        {app.total_duration_seconds}s
                      </p>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="px-6 py-3 bg-[#0a0e27] border-t border-[#1e2339]">
            <p className="text-xs text-[#717579]">
              Mostrados {apps.length} de {apps.length} aplicaciones más utilizadas
            </p>
          </div>
        </>
      )}
    </div>
  );
}
