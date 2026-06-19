import { useMemo } from "react";
import type { Service } from "@/api/types";

export function useGlobalMetrics(services: Service[]) {
  const running = useMemo(
    () => services.filter((s) => s.status === "running"),
    [services],
  );

  const totals = useMemo(() => {
    let cpu = 0;
    let mem = 0;
    for (const svc of running) {
      if (svc.metrics) {
        cpu += svc.metrics.cpu_percent;
        mem += svc.metrics.memory_mb;
      }
    }
    return {
      cpu: Math.round(cpu * 10) / 10,
      mem: Math.round(mem * 10) / 10,
    };
  }, [running]);

  return {
    ...totals,
    runningCount: running.length,
    totalCount: services.length,
  };
}

export function GlobalMetricsBar({ services }: { services: Service[] }) {
  const { cpu, mem, runningCount, totalCount } = useGlobalMetrics(services);
  if (totalCount === 0) return null;

  return (
    <div className="flex items-center gap-3 text-xs tabular-nums">
      <span className="text-text-tertiary">
        CPU{" "}
        <span className="text-text-muted">
          {cpu % 1 === 0 ? String(cpu) : cpu.toFixed(1)}%
        </span>
      </span>
      <span className="text-text-tertiary">·</span>
      <span className="text-text-tertiary">
        RAM{" "}
        <span className="text-text-muted">
          {mem % 1 === 0 ? String(mem) : mem.toFixed(1)} MB
        </span>
      </span>
      <span className="text-text-tertiary">·</span>
      <span className="text-text-tertiary">
        <span className="text-success">{runningCount}</span>/{totalCount}
      </span>
    </div>
  );
}
