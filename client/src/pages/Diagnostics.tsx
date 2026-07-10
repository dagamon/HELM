import { useEffect, useRef, useState } from "react";
import { Cpu, MemoryStick, HardDrive, Activity, Server, Clock } from "lucide-react";
import { api } from "@/api/client";
import type { Diagnostics as Diag } from "@/api/types";

function fmtBytes(n: number): string {
  if (!n) return "0 B";
  const u = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), u.length - 1);
  return `${(n / 1024 ** i).toFixed(i === 0 ? 0 : 1)} ${u[i]}`;
}

function fmtUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function barColor(pct: number): string {
  if (pct >= 85) return "bg-danger";
  if (pct >= 60) return "bg-warning";
  return "bg-accent";
}

function Bar({ pct }: { pct: number }) {
  return (
    <div className="h-1.5 w-full rounded-full bg-surface-hover overflow-hidden">
      <div
        className={`h-full rounded-full transition-all duration-500 ${barColor(pct)}`}
        style={{ width: `${Math.min(100, Math.max(0, pct))}%` }}
      />
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  sub,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div className="rounded-xl border border-border bg-surface p-4">
      <div className="flex items-center gap-2 text-xs uppercase tracking-wide text-text-muted">
        <Icon className="w-3.5 h-3.5 text-accent" />
        {label}
      </div>
      <div className="mt-2 text-2xl font-semibold tabular-nums">{value}</div>
      {sub && <div className="text-xs text-text-muted mt-0.5">{sub}</div>}
    </div>
  );
}

export function Diagnostics() {
  const [diag, setDiag] = useState<Diag | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const d = await api.diagnostics();
        if (alive) {
          setDiag(d);
          setError(null);
        }
      } catch (e) {
        if (alive) setError(e instanceof Error ? e.message : "failed to load");
      }
    };
    tick();
    timer.current = setInterval(tick, 2000);
    return () => {
      alive = false;
      if (timer.current) clearInterval(timer.current);
    };
  }, []);

  if (error && !diag) {
    return <div className="text-sm text-danger">Failed to load diagnostics: {error}</div>;
  }
  if (!diag) {
    return <div className="text-sm text-text-muted">Loading diagnostics…</div>;
  }

  const { host, helm, processes } = diag;
  const memPct = host.mem_total_bytes
    ? (host.mem_used_bytes / host.mem_total_bytes) * 100
    : 0;
  const helmMemBytes = helm.memory_mb * 1024 * 1024;
  const helmMemShare = host.mem_total_bytes
    ? (helmMemBytes / host.mem_total_bytes) * 100
    : 0;

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      <div>
        <h1 className="text-lg font-semibold">Diagnostics</h1>
        <p className="text-text-muted text-sm mt-1">
          Live host resources and how much HELM's managed services account for. Updates every 2s.
        </p>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={Cpu}
          label="CPU"
          value={`${host.cpu_usage.toFixed(1)}%`}
          sub={`${host.core_count} cores · ${host.cpu_brand || "unknown"}`}
        />
        <StatCard
          icon={MemoryStick}
          label="Memory"
          value={`${memPct.toFixed(0)}%`}
          sub={`${fmtBytes(host.mem_used_bytes)} / ${fmtBytes(host.mem_total_bytes)}`}
        />
        <StatCard
          icon={Server}
          label="HELM services"
          value={`${helm.running_count}/${helm.service_count}`}
          sub={`${helm.cpu_percent.toFixed(1)}% cpu · ${fmtBytes(helmMemBytes)}`}
        />
        <StatCard
          icon={Clock}
          label="Host uptime"
          value={fmtUptime(host.uptime_seconds)}
          sub={`HELM up ${fmtUptime(helm.uptime_seconds)}`}
        />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Per-core CPU */}
        <section className="rounded-xl border border-border bg-surface p-4 space-y-3">
          <h2 className="text-xs uppercase tracking-wide text-text-muted flex items-center gap-2">
            <Cpu className="w-3.5 h-3.5 text-accent" /> Per-core load
          </h2>
          <div className="grid grid-cols-2 gap-x-5 gap-y-2.5">
            {host.cores.map((c, i) => (
              <div key={c.name || i} className="space-y-1">
                <div className="flex justify-between text-xs text-text-muted">
                  <span>{c.name || `cpu${i}`}</span>
                  <span className="tabular-nums">{c.usage.toFixed(0)}%</span>
                </div>
                <Bar pct={c.usage} />
              </div>
            ))}
          </div>
        </section>

        {/* Memory + disks */}
        <section className="rounded-xl border border-border bg-surface p-4 space-y-4">
          <div className="space-y-2">
            <h2 className="text-xs uppercase tracking-wide text-text-muted flex items-center gap-2">
              <MemoryStick className="w-3.5 h-3.5 text-accent" /> Memory
            </h2>
            <div className="flex justify-between text-xs text-text-muted">
              <span>HELM share</span>
              <span className="tabular-nums">{helmMemShare.toFixed(1)}%</span>
            </div>
            <Bar pct={memPct} />
            {host.swap_total_bytes > 0 && (
              <div className="text-xs text-text-muted">
                Swap {fmtBytes(host.swap_used_bytes)} / {fmtBytes(host.swap_total_bytes)}
              </div>
            )}
          </div>

          <div className="space-y-2.5">
            <h2 className="text-xs uppercase tracking-wide text-text-muted flex items-center gap-2">
              <HardDrive className="w-3.5 h-3.5 text-accent" /> Disks
            </h2>
            {host.disks.map((d, i) => {
              const used = d.total_bytes - d.available_bytes;
              const pct = d.total_bytes ? (used / d.total_bytes) * 100 : 0;
              return (
                <div key={d.mount || i} className="space-y-1">
                  <div className="flex justify-between text-xs text-text-muted">
                    <span className="truncate">{d.mount || d.name}</span>
                    <span className="tabular-nums">
                      {fmtBytes(used)} / {fmtBytes(d.total_bytes)}
                    </span>
                  </div>
                  <Bar pct={pct} />
                </div>
              );
            })}
            {host.disks.length === 0 && (
              <p className="text-xs text-text-muted">No disks reported</p>
            )}
          </div>
        </section>
      </div>

      {/* Process breakdown */}
      <section className="rounded-xl border border-border bg-surface overflow-hidden">
        <h2 className="text-xs uppercase tracking-wide text-text-muted flex items-center gap-2 px-4 py-3 border-b border-border">
          <Activity className="w-3.5 h-3.5 text-accent" /> Managed processes
        </h2>
        {processes.length === 0 ? (
          <p className="text-xs text-text-muted px-4 py-4">No running managed processes</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-xs text-text-muted text-left">
                <th className="px-4 py-2 font-medium">Name</th>
                <th className="px-4 py-2 font-medium">Type</th>
                <th className="px-4 py-2 font-medium">PID</th>
                <th className="px-4 py-2 font-medium text-right">CPU</th>
                <th className="px-4 py-2 font-medium text-right">Memory</th>
              </tr>
            </thead>
            <tbody>
              {processes.map((p) => (
                <tr key={`${p.entity_type}_${p.entity_id}`} className="border-t border-border/60">
                  <td className="px-4 py-2">{p.name}</td>
                  <td className="px-4 py-2 text-text-muted">{p.entity_type}</td>
                  <td className="px-4 py-2 tabular-nums text-text-muted">{p.pid}</td>
                  <td className="px-4 py-2 tabular-nums text-right">{p.cpu_percent.toFixed(1)}%</td>
                  <td className="px-4 py-2 tabular-nums text-right">
                    {fmtBytes(p.memory_mb * 1024 * 1024)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
