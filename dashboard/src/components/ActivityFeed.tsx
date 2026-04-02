import { useActivityStream } from '../hooks/useActivityStream';

export function ActivityFeed() {
  const { data, isConnected, error } = useActivityStream();

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-xl font-bold text-gray-800">Live Activity Stream</h3>
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`}></div>
          <span className={`text-xs font-medium ${isConnected ? 'text-green-600' : 'text-red-600'}`}>
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-yellow-50 border border-yellow-200 rounded-md">
          <p className="text-xs text-yellow-800">{error}</p>
        </div>
      )}

      <div className="space-y-2 max-h-96 overflow-y-auto">
        {!data || !data.activities || data.activities.length === 0 ? (
          <p className="text-gray-500 py-6 text-center">Waiting for activity data...</p>
        ) : (
          data.activities.map((activity, index) => (
            <div
              key={`${activity.device_id}-${index}`}
              className={`p-3 rounded-md border transition-all ${
                activity.is_idle
                  ? 'bg-gray-50 border-gray-300'
                  : 'bg-blue-50 border-blue-200'
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`inline-block px-2 py-1 rounded text-xs font-semibold ${
                      activity.is_idle
                        ? 'bg-gray-200 text-gray-700'
                        : 'bg-blue-200 text-blue-700'
                    }`}>
                      {activity.is_idle ? '🛌 Idle' : '🔴 Active'}
                    </span>
                    <span className="text-xs font-medium text-gray-600">
                      {activity.device_id.substring(0, 8)}...
                    </span>
                  </div>
                  <p className="font-semibold text-gray-800 text-sm truncate">
                    {activity.app}
                  </p>
                  <p className="text-xs text-gray-600 truncate">
                    {activity.title || '(No title)'}
                  </p>
                  <p className="text-xs text-gray-400 mt-1">
                    {new Date(activity.last_seen).toLocaleTimeString()}
                  </p>
                </div>
                <div className={`w-2 h-2 rounded-full flex-shrink-0 ${
                  activity.is_live ? 'bg-green-500' : 'bg-gray-400'
                }`}></div>
              </div>
            </div>
          ))
        )}
      </div>

      {data && data.activities && data.activities.length > 0 && (
        <div className="mt-3 text-xs text-gray-400 text-center">
          Last update: {new Date(data.timestamp).toLocaleTimeString()}
        </div>
      )}
    </div>
  );
}
