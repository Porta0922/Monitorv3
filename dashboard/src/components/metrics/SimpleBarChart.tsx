interface BarChartProps {
  data: any[];
  dataKey: string;
  label: string;
  color?: string;
}

export function SimpleBarChart({
  data,
  dataKey,
  label,
  color = '#29B6F6',
}: BarChartProps) {
  if (!data || data.length === 0) return null;

  const width = 500;
  const height = 250;
  const padding = 40;
  const graphWidth = width - padding * 2;
  const graphHeight = height - padding * 2;

  // Find max value
  const values = data.map((d) => d[dataKey] || 0);
  const maxValue = Math.max(...values, 1);

  // Bar dimensions
  const barWidth = graphWidth / data.length * 0.8;
  const barGap = graphWidth / data.length * 0.2;

  // Create grid lines
  const gridLines = [];
  for (let i = 0; i <= 4; i++) {
    const y = padding + graphHeight - (i / 4) * graphHeight;
    const value = (i / 4) * maxValue;
    gridLines.push(
      <g key={`grid-${i}`}>
        <line
          x1={padding}
          y1={y}
          x2={width - padding}
          y2={y}
          stroke="#1a3a52"
          strokeWidth="1"
          strokeDasharray="4"
        />
        <text
          x={padding - 5}
          y={y + 4}
          textAnchor="end"
          fontSize="11"
          fill="#8899bb"
        >
          {Math.round(value)}
        </text>
      </g>
    );
  }

  return (
    <div className="h-full w-full">
      <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" className="h-full w-full rounded bg-[#0a122a]/60">
        {/* Grid */}
        {gridLines}

        {/* Axes */}
        <line
          x1={padding}
          y1={padding}
          x2={padding}
          y2={height - padding}
          stroke="#FF9800"
          strokeWidth="2"
        />
        <line
          x1={padding}
          y1={height - padding}
          x2={width - padding}
          y2={height - padding}
          stroke="#FF9800"
          strokeWidth="2"
        />

        {/* Bars */}
        {data.map((d, i) => {
          const value = d[dataKey] || 0;
          const barHeight = (value / maxValue) * graphHeight;
          const x = padding + i * (barWidth + barGap) + barGap / 2;
          const y = height - padding - barHeight;

          return (
            <g key={`bar-${i}`}>
              <rect
                x={x}
                y={y}
                width={barWidth}
                height={barHeight}
                fill={color}
                opacity="0.7"
                rx="4"
              />
              {/* Hour label */}
              <text
                x={x + barWidth / 2}
                y={height - padding + 20}
                textAnchor="middle"
                fontSize="11"
                fill="#8899bb"
              >
                {d.hour || i}h
              </text>
            </g>
          );
        })}

        {/* Y-axis label */}
        <text
          x={15}
          y={padding - 10}
          fontSize="12"
          fill="#00d9ff"
          fontWeight="bold"
        >
          {label}
        </text>
      </svg>
    </div>
  );
}
