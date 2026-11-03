import { useActivityStream } from '../hooks/useActivityStream';

export function LiveActivityTable() {
  const { data, isConnected, error } = useActivityStream();

  return (
    <div className="bg-[var(--palette-f5f7f6)] rounded-xl border border-[var(--palette-dfe7e3)] overflow-hidden shadow-lg">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-bold text-[var(--palette-53685c)]">🔴 En Vivo</h3>
          <div className={`w-3 h-3 rounded-full animate-pulse ${
            isConnected ? 'bg-[var(--palette-17715a)]' : 'bg-red-500'
          }`}></div>
        </div>
        <span className={`text-xs font-medium px-2 py-1 rounded ${
          isConnected
            ? 'bg-[var(--palette-17715a)]/10 text-[var(--palette-17715a)]'
            : 'bg-red-500/10 text-red-500'
        }`}>
          {isConnected ? 'Conectado' : 'Desconectado'}
        </span>
      </div>

      {/* Error Message */}
      {error && (
        <div className="px-6 py-3 bg-red-500/10 border-b border-red-500/20">
          <p className="text-xs text-red-700">{error}</p>
        </div>
      )}

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-sm font-sans">
          <thead>
            <tr className="border-b border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)]">
              <th className="px-6 py-3 text-left text-[var(--palette-53685c)] font-semibold">Estado</th>
              <th className="px-6 py-3 text-left text-[var(--palette-53685c)] font-semibold">Aplicación</th>
              <th className="px-6 py-3 text-left text-[var(--palette-53685c)] font-semibold">Título</th>
              <th className="px-6 py-3 text-left text-[var(--palette-53685c)] font-semibold">Dispositivo</th>
              <th className="px-6 py-3 text-right text-[var(--palette-53685c)] font-semibold">Hora</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[var(--palette-1e2339)]">
            {!data || !data.activities || data.activities.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-6 py-8 text-center text-[var(--palette-53685c)]">
                  Esperando datos en vivo...
                </td>
              </tr>
            ) : (
              data.activities.map((activity, index) => (
                <tr
                  key={`${activity.device_id}-${index}`}
                  className="hover:bg-[var(--palette-f5f7f6)] transition-colors"
                >
                  {/* Estado */}
                  <td className="px-6 py-3">
                    <div className="flex items-center gap-2">
                      <div className={`w-2 h-2 rounded-full ${
                        activity.is_idle ? 'bg-[var(--palette-f5f7f6)]' : 'bg-[var(--palette-31764d)]'
                      }`}></div>
                      <span className={`text-xs font-bold ${
                        activity.is_idle
                          ? 'text-[var(--palette-53685c)]'
                          : 'text-[var(--palette-31764d)]'
                      }`}>
                        {activity.is_idle ? 'IDLE' : 'ACTIVE'}
                      </span>
                    </div>
                  </td>

                  {/* Aplicación */}
                  <td className="px-6 py-3">
                    <span className="text-[var(--palette-17715a)] font-semibold">{activity.app}</span>
                  </td>

                  {/* Título */}
                  <td className="px-6 py-3">
                    <span className="text-[var(--palette-53685c)] truncate max-w-xs block">
                      {activity.title || '(sin título)'}
                    </span>
                  </td>

                  {/* Dispositivo */}
                  <td className="px-6 py-3">
                    <span className="text-[var(--palette-53685c)] text-xs">
                      {activity.device_id.substring(0, 8)}...
                    </span>
                  </td>

                  {/* Hora */}
                  <td className="px-6 py-3 text-right">
                    <span className="text-[var(--palette-31764d)] text-xs">
                      {new Date(activity.last_seen).toLocaleTimeString()}
                    </span>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Footer */}
      {data && (
        <div className="px-6 py-3 border-t border-[var(--palette-dfe7e3)] bg-[var(--palette-f5f7f6)] text-xs text-[var(--palette-53685c)] text-right">
          Actualizado: {new Date(data.timestamp).toLocaleTimeString()}
        </div>
      )}
    </div>
  );
}
