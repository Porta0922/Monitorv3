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
    cyan:   { border: 'border-[var(--palette-dfe7e3)]/30', accent: 'text-[var(--palette-53685c)]', value: 'text-[var(--palette-17715a)]' },
    green:  { border: 'border-[var(--palette-dfe7e3)]/30', accent: 'text-[var(--palette-53685c)]', value: 'text-[var(--palette-31764d)]' },
    orange: { border: 'border-[var(--palette-dfe7e3)]/30', accent: 'text-[var(--palette-53685c)]', value: 'text-[var(--palette-97621b)]' },
    red:    { border: 'border-[var(--palette-dfe7e3)]/30', accent: 'text-[var(--palette-53685c)]', value: 'text-[var(--palette-53685c)]' },
    yellow: { border: 'border-[var(--palette-dfe7e3)]/30', accent: 'text-[var(--palette-53685c)]', value: 'text-[var(--palette-926a1b)]' },
  };

  const styles = colorMap[color] ?? colorMap['cyan'];
  const trendChange = trend ? trend.current - trend.previous : 0;
  const trendPercent = trend && trend.previous > 0 ? Math.round((trendChange / trend.previous) * 100) : 0;
  const isPositive = trendChange >= 0;

  return (
    <article className={`rounded-2xl border ${styles.border} bg-[var(--surface)] px-4 py-3 shadow-sm`}>
      <p className="font-sans text-[12px] uppercase tracking-normal text-[var(--palette-53685c)]">{title}</p>
      <div className="mt-2 flex items-baseline gap-1.5">
        <span className={`font-display text-[22px] font-bold leading-none ${styles.value}`}>
          {value.toLocaleString()}
        </span>
        <span className="font-sans text-[12px] text-[var(--palette-53685c)]">{unit}</span>
      </div>
      {trend ? (
        <div className="mt-1.5 flex items-center gap-1">
          <span className={`font-sans text-[12px] ${isPositive ? 'text-[var(--palette-31764d)]' : 'text-[var(--palette-b2433d)]'}`}>
            {isPositive ? '↑' : '↓'} {Math.abs(trendPercent)}%
          </span>
          <span className="font-sans text-[12px] text-[var(--palette-53685c)]">vs ayer</span>
        </div>
      ) : (
        <div className="mt-1.5 h-[18px]" />
      )}
    </article>
  );
}
