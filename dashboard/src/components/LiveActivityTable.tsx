import { useActivityStream } from '../hooks/useActivityStream';

export function LiveActivityTable() {
  const { data, isConnected, error } = useActivityStream();

  return (
    <div className="bg-[#131829] rounded-xl border border-[#1e2339] overflow-hidden shadow-lg">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-[#1e2339] bg-[#0a0e27]">
        <div className="flex items-center gap-3">
          <h3 className="text-lg font-bold text-[#e4e6eb]">🔴 En Vivo</h3>
          <div className={`w-3 h-3 rounded-full animate-pulse ${
            isConnected ? 'bg-[#00d9ff]' : 'bg-red-500'
          }`}></div>
        </div>
        <span className={`text-xs font-medium px-2 py-1 rounded ${
          isConnected 
            ? 'bg-[#00d9ff]/10 text-[#00d9ff]' 
            : 'bg-red-500/10 text-red-500'
        }`}>
          {isConnected ? 'Conectado' : 'Desconectado'}
        </span>
      </div>

      {/* Error Message */}
      {error && (
        <div className="px-6 py-3 bg-red-500/10 border-b border-red-500/20">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-sm font-mono">
          <thead>
            <tr className="border-b border-[#1e2339] bg-[#0a0e27]">
              <th className="px-6 py-3 text-left text-[#a0a5b2] font-semibold">Estado</th>
              <th className="px-6 py-3 text-left text-[#a0a5b2] font-semibold">Aplicación</th>
              <th className="px-6 py-3 text-left text-[#a0a5b2] font-semibold">Título</th>
              <th className="px-6 py-3 text-left text-[#a0a5b2] font-semibold">Dispositivo</th>
              <th className="px-6 py-3 text-right text-[#a0a5b2] font-semibold">Hora</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-[#1e2339]">
            {!data || !data.activities || data.activities.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-6 py-8 text-center text-[#717579]">
                  Esperando datos en vivo...
                </td>
              </tr>
            ) : (
              data.activities.map((activity, index) => (
                <tr
                  key={`${activity.device_id}-${index}`}
                  className="hover:bg-[#1a1f3a] transition-colors"
                >
                  {/* Estado */}
                  <td className="px-6 py-3">
                    <div className="flex items-center gap-2">
                      <div className={`w-2 h-2 rounded-full ${
                        activity.is_idle ? 'bg-[#717579]' : 'bg-[#00ff88]'
                      }`}></div>
                      <span className={`text-xs font-bold ${
                        activity.is_idle 
                          ? 'text-[#717579]' 
                          : 'text-[#00ff88]'
                      }`}>
                        {activity.is_idle ? 'IDLE' : 'ACTIVE'}
                      </span>
                    </div>
                  </td>

                  {/* Aplicación */}
                  <td className="px-6 py-3">
                    <span className="text-[#00d9ff] font-semibold">{activity.app}</span>
                  </td>

                  {/* Título */}
                  <td className="px-6 py-3">
                    <span className="text-[#e4e6eb] truncate max-w-xs block">
                      {activity.title || '(sin título)'}
                    </span>
                  </td>

                  {/* Dispositivo */}
                  <td className="px-6 py-3">
                    <span className="text-[#a0a5b2] text-xs">
                      {activity.device_id.substring(0, 8)}...
                    </span>
                  </td>

                  {/* Hora */}
                  <td className="px-6 py-3 text-right">
                    <span className="text-[#00ff88] text-xs">
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
        <div className="px-6 py-3 border-t border-[#1e2339] bg-[#0a0e27] text-xs text-[#717579] text-right">
          Actualizado: {new Date(data.timestamp).toLocaleTimeString()}
        </div>
      )}
    </div>
  );
}
