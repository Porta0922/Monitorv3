import { useTopApps } from '../hooks/useTopApps';

export function TopAppsTable() {
  const { apps, isLoading, error } = useTopApps();

  if (isLoading) {
    return (
      <div className="bg-[var(--palette-f5f7f6)] rounded-xl border border-[var(--palette-dfe7e3)] p-8">
        <p className="text-[var(--palette-53685c)]">Cargando aplicaciones...</p>
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

  return (
    <div className="bg-[var(--surface)] rounded-xl border border-[var(--palette-dfe7e3)] overflow-hidden shadow-sm">
      <div className="px-6 py-4 border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
        <h2 className="text-xl font-bold text-[var(--palette-53685c)]">🚀 Top 6 Aplicaciones (Últimos 7 Días)</h2>
      </div>

      {apps.length === 0 ? (
        <div className="px-6 py-8">
          <p className="text-[var(--palette-53685c)]">Sin datos de aplicaciones</p>
        </div>
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="bg-[var(--palette-f5f7f6)] border-b border-[var(--palette-dfe7e3)]">
                  <th className="px-6 py-3 text-left text-xs font-bold text-[var(--palette-17715a)] uppercase tracking-wider">Rank</th>
                  <th className="px-6 py-3 text-left text-xs font-bold text-[var(--palette-17715a)] uppercase tracking-wider">Aplicación</th>
                  <th className="px-6 py-3 text-right text-xs font-bold text-[var(--palette-31764d)] uppercase tracking-wider">Horas</th>
                  <th className="px-6 py-3 text-right text-xs font-bold text-[var(--palette-31764d)] uppercase tracking-wider">Segundos</th>
                </tr>
              </thead>
              <tbody>
                {apps.map((app, index) => (
                  <tr
                    key={app.app_name}
                    className="border-b border-[var(--palette-dfe7e3)] hover:bg-[var(--palette-f5f7f6)] transition-colors"
                  >
                    <td className="px-6 py-3">
                      <span className="inline-flex items-center justify-center w-7 h-7 rounded-full bg-[var(--palette-17715a)]/10 text-[var(--palette-17715a)] font-sans font-bold text-sm">
                        {index + 1}
                      </span>
                    </td>
                    <td className="px-6 py-3">
                      <p className="font-sans text-[var(--palette-53685c)]">{app.app_name}</p>
                    </td>
                    <td className="px-6 py-3 text-right">
                      <p className="font-sans text-[var(--palette-31764d)] font-semibold">
                        {(+app.total_duration_hours).toFixed(2)}h
                      </p>
                    </td>
                    <td className="px-6 py-3 text-right">
                      <p className="font-sans text-[var(--palette-53685c)] text-sm">
                        {app.total_duration_seconds}s
                      </p>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="px-6 py-3 bg-[var(--palette-f5f7f6)] border-t border-[var(--palette-dfe7e3)]">
            <p className="text-xs text-[var(--palette-53685c)]">
              Mostrados {apps.length} de {apps.length} aplicaciones más utilizadas
            </p>
          </div>
        </>
      )}
    </div>
  );
}
