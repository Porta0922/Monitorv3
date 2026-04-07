interface MetricTrend {
  current: number;
  previous: number;
  unit: string;
  label: string;
}

interface MetricCardProps {
  title: string;
  value: number;
  unit: string;
  color: 'cyan' | 'green' | 'blue' | 'red' | 'yellow';
  icon: string;
  trend?: MetricTrend;
}

export function MetricCard({
  title,
  value,
  unit,
  color,
  icon,
  trend,
}: MetricCardProps) {
  const colorMap = {
    cyan: {
      bg: 'bg-cyan-900/20',
      border: 'border-cyan-500/30',
      text: 'text-cyan-400',
      value: 'text-cyan-300',
    },
    green: {
      bg: 'bg-green-900/20',
      border: 'border-green-500/30',
      text: 'text-green-400',
      value: 'text-green-300',
    },
    blue: {
      bg: 'bg-blue-900/20',
      border: 'border-blue-500/30',
      text: 'text-blue-400',
      value: 'text-blue-300',
    },
    red: {
      bg: 'bg-red-900/20',
      border: 'border-red-500/30',
      text: 'text-red-400',
      value: 'text-red-300',
    },
    yellow: {
      bg: 'bg-yellow-900/20',
      border: 'border-yellow-500/30',
      text: 'text-yellow-400',
      value: 'text-yellow-300',
    },
  };

  const styles = colorMap[color];
  const trendChange = trend ? trend.current - trend.previous : 0;
  const trendPercent = trend ? Math.round(((trendChange / trend.previous) * 100)) : 0;
  const isPositive = trendChange >= 0;

  return (
    <div className={`${styles.bg} border ${styles.border} rounded-lg p-4 transition hover:shadow-lg`}>
      <div className="flex items-start justify-between">
        <div>
          <p className={`text-sm font-medium ${styles.text} mb-2`}>{title}</p>
          <div className="flex items-baseline gap-2">
            <span className={`text-2xl font-bold ${styles.value}`}>{value.toLocaleString()}</span>
            <span className={`text-xs ${styles.text}`}>{unit}</span>
          </div>
          {trend && (
            <div className="mt-3 flex items-center gap-1">
              <span className={`text-xs font-semibold ${isPositive ? 'text-green-400' : 'text-red-400'}`}>
                {isPositive ? '↑' : '↓'} {Math.abs(trendPercent)}%
              </span>
              <span className="text-xs text-[#8899bb]">vs ayer</span>
            </div>
          )}
        </div>
        <span className="text-3xl opacity-60">{icon}</span>
      </div>
    </div>
  );
}
