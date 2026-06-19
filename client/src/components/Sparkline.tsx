import { useMemo } from "react";

interface Props {
  data: number[];
  width?: number;
  height?: number;
  color?: string;
  label?: string;
  suffix?: string;
  showValue?: boolean;
}

let idCounter = 0;

export function Sparkline({
  data,
  width = 100,
  height = 32,
  color = "#3b82f6",
  label,
  suffix,
  showValue = true,
}: Props) {
  const gradientId = useMemo(() => `spark-grad-${++idCounter}`, []);

  if (data.length < 2) return null;

  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;
  const padY = 2;

  const points = data.map((v, i) => {
    const x = (i / (data.length - 1)) * width;
    const y = height - ((v - min) / range) * (height - padY * 2) - padY;
    return [x, y] as const;
  });

  const linePath = points.map((p, i) => `${i === 0 ? "M" : "L"}${p[0]},${p[1]}`).join(" ");
  const areaPath = `${linePath} L${width},${height} L0,${height} Z`;

  const last = data[data.length - 1];
  const formatted = last % 1 === 0 ? String(last) : last.toFixed(1);

  return (
    <div className="flex items-center gap-2" title={label}>
      <svg
        width={width}
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        className="overflow-visible shrink-0"
      >
        <defs>
          <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.3" />
            <stop offset="100%" stopColor={color} stopOpacity="0.02" />
          </linearGradient>
        </defs>
        <path d={areaPath} fill={`url(#${gradientId})`} />
        <path
          d={linePath}
          fill="none"
          stroke={color}
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        {/* Current value dot */}
        <circle
          cx={points[points.length - 1][0]}
          cy={points[points.length - 1][1]}
          r="2"
          fill={color}
        />
      </svg>
      {showValue && (
        <span className="text-xs font-medium text-text tabular-nums whitespace-nowrap">
          {formatted}
          {suffix}
        </span>
      )}
    </div>
  );
}
