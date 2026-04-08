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
  color: 'cyan' | 'green' | 'orange' | 'red' | 'yellow';
  trend?: MetricTrend;
}

export function MetricCard({
  title,
  value,
  unit,
  color,
  trend,
}: MetricCardProps) {
  const colorMap: Record<string, { border: string; accent: string; value: string }> = {
    cyan:   { border: 'border-[#00d9ff]/30', accent: 'text-[#8ea0cf]', value: 'text-[#00d9ff]' },
    green:  { border: 'border-[#00ff88]/30', accent: 'text-[#8ea0cf]', value: 'text-[#00ff88]' },
    orange: { border: 'border-[#ff9f1a]/30', accent: 'text-[#8ea0cf]', value: 'text-[#ff9f1a]' },
    red:    { border: 'border-[#ff5f7a]/30', accent: 'text-[#8ea0cf]', value: 'text-[#ff8ea0]' },
    yellow: { border: 'border-[#ffd54a]/30', accent: 'text-[#8ea0cf]', value: 'text-[#ffd54a]' },
  };

  const styles = colorMap[color] ?? colorMap['cyan'];
  const trendChange = trend ? trend.current - trend.previous : 0;
  const trendPercent = trend && trend.previous > 0 ? Math.round((trendChange / trend.previous) * 100) : 0;
  const isPositive = trendChange >= 0;

  return (
    <article className={`rounded-2xl border ${styles.border} bg-[linear-gradient(160deg,#0f1d43,#0b1329)] px-4 py-3 shadow-[0_10px_22px_rgba(0,0,0,0.35)]`}>
      <p className="font-mono text-[10px] uppercase tracking-[0.2em] text-[#8ea0cf]">{title}</p>
      <div className="mt-2 flex items-baseline gap-1.5">
        <span className={`font-display text-[22px] font-bold leading-none ${styles.value}`}>
          {value.toLocaleString()}
        </span>
        <span className="font-mono text-[10px] text-[#5a6a90]">{unit}</span>
      </div>
      {trend ? (
        <div className="mt-1.5 flex items-center gap-1">
          <span className={`font-mono text-[10px] ${isPositive ? 'text-[#00ff88]' : 'text-[#ff5f7a]'}`}>
            {isPositive ? '↑' : '↓'} {Math.abs(trendPercent)}%
          </span>
          <span className="font-mono text-[10px] text-[#5a6a90]">vs ayer</span>
        </div>
      ) : (
        <div className="mt-1.5 h-[18px]" />
      )}
    </article>
  );
}
