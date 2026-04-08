interface LineChartProps {
  data: any[];
  dataKey: string;
  label: string;
  color?: string;
}

export function SimpleLineChart({
  data,
  dataKey,
  label,
  color = '#00d9ff',
}: LineChartProps) {
  if (!data || data.length === 0) return null;

  const width = 500;
  const height = 250;
  const padding = 40;
  const graphWidth = width - padding * 2;
  const graphHeight = height - padding * 2;

  // Find min and max values
  const values = data.map((d) => d[dataKey] || 0);
  const maxValue = Math.max(...values, 1);
  const minValue = Math.min(...values, 0);
  const range = maxValue - minValue || 1;

  // Create SVG points
  const points = data.map((d, i) => {
    const x = padding + (i / (data.length - 1 || 1)) * graphWidth;
    const normalizedValue = (d[dataKey] - minValue) / range;
    const y = padding + graphHeight - normalizedValue * graphHeight;
    return { x, y, value: d[dataKey] };
  });

  // Create path
  const pathData = points
    .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`)
    .join(' ');

  // Create grid lines
  const gridLines = [];
  for (let i = 0; i <= 4; i++) {
    const y = padding + (i / 4) * graphHeight;
    const value = maxValue - (i / 4) * range;
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
          stroke="#4CAF50"
          strokeWidth="2"
        />
        <line
          x1={padding}
          y1={height - padding}
          x2={width - padding}
          y2={height - padding}
          stroke="#4CAF50"
          strokeWidth="2"
        />

        {/* Line */}
        <path
          d={pathData}
          stroke={color}
          strokeWidth="2.5"
          fill="none"
          opacity="0.8"
        />

        {/* Points */}
        {points.map((p, i) => (
          <circle key={`point-${i}`} cx={p.x} cy={p.y} r="4" fill={color} opacity="0.7" />
        ))}

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
