import { useEffect, useState } from "react";
import type { Service } from "@/api/types";
import { Sparkline } from "./Sparkline";

const MAX_HISTORY = 60;

export function MetricsPanel({ service }: { service: Service }) {
  const isRunning = service.status === "running";

  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memHistory, setMemHistory] = useState<number[]>([]);

  useEffect(() => {
    if (service.metrics) {
      setCpuHistory((prev) => [...prev, service.metrics!.cpu_percent].slice(-MAX_HISTORY));
      setMemHistory((prev) => [...prev, service.metrics!.memory_mb].slice(-MAX_HISTORY));
    }
  }, [service.metrics]);

  useEffect(() => {
    if (!isRunning) {
      setCpuHistory([]);
      setMemHistory([]);
    }
  }, [isRunning]);

  if (!isRunning) {
    return (
      <div className="bg-surface rounded-lg px-4 py-3">
        <p className="text-xs text-text-tertiary">Metrics available when service is running</p>
      </div>
    );
  }

  const lastCpu = cpuHistory.length > 0 ? cpuHistory[cpuHistory.length - 1] : null;
  const lastMem = memHistory.length > 0 ? memHistory[memHistory.length - 1] : null;

  return (
    <div className="grid grid-cols-2 gap-2">
      {/* CPU */}
      <div className="bg-surface rounded-lg px-4 py-3">
        <div className="flex items-center justify-between mb-3">
          <span className="text-[10px] text-text-tertiary uppercase tracking-wide">CPU</span>
          {lastCpu !== null && (
            <span className="text-sm font-semibold text-text-muted tabular-nums">
              {lastCpu % 1 === 0 ? String(lastCpu) : lastCpu.toFixed(1)}%
            </span>
          )}
        </div>
        {cpuHistory.length >= 2 ? (
          <Sparkline
            data={cpuHistory}
            width={300}
            height={44}
            color="#8b8b93"
            label="CPU %"
            showValue={false}
          />
        ) : (
          <div className="text-xs text-text-tertiary">Collecting...</div>
        )}
      </div>

      {/* Memory */}
      <div className="bg-surface rounded-lg px-4 py-3">
        <div className="flex items-center justify-between mb-3">
          <span className="text-[10px] text-text-tertiary uppercase tracking-wide">Memory</span>
          {lastMem !== null && (
            <span className="text-sm font-semibold text-text-muted tabular-nums">
              {lastMem % 1 === 0 ? String(lastMem) : lastMem.toFixed(1)} MB
            </span>
          )}
        </div>
        {memHistory.length >= 2 ? (
          <Sparkline
            data={memHistory}
            width={300}
            height={44}
            color="#6b6b73"
            label="RAM MB"
            showValue={false}
          />
        ) : (
          <div className="text-xs text-text-tertiary">Collecting...</div>
        )}
      </div>
    </div>
  );
}
