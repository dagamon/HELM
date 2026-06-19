import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Play, Square, RotateCw, Trash2, Terminal, ExternalLink } from "lucide-react";
import type { Service } from "@/api/types";
import { useServices } from "@/store/services";
import { Sparkline } from "./Sparkline";

const MAX_HISTORY = 30;

const STATUS_DOT: Record<string, string> = {
  running: "bg-success",
  stopped: "bg-text-tertiary",
  crashed: "bg-danger",
  error: "bg-danger",
};

export function ServiceCard({ service }: { service: Service }) {
  const { start, stop, restart, remove } = useServices();
  const navigate = useNavigate();
  const isRunning = service.status === "running";

  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memHistory, setMemHistory] = useState<number[]>([]);

  useEffect(() => {
    if (service.metrics) {
      setCpuHistory((prev) =>
        [...prev, service.metrics!.cpu_percent].slice(-MAX_HISTORY),
      );
      setMemHistory((prev) =>
        [...prev, service.metrics!.memory_mb].slice(-MAX_HISTORY),
      );
    }
  }, [service.metrics]);

  useEffect(() => {
    if (!isRunning) {
      setCpuHistory([]);
      setMemHistory([]);
    }
  }, [isRunning]);

  const handleAction = async (
    e: React.MouseEvent,
    action: () => Promise<void>,
  ) => {
    e.stopPropagation();
    try {
      await action();
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div
      onClick={() => navigate(`/services/${service.id}`)}
      className="group bg-surface border border-border rounded-xl p-4 hover:bg-surface-hover hover:border-border-hover transition-colors cursor-pointer"
    >
      {/* Row 1: Status dot + Name */}
      <div className="flex items-center gap-2.5 mb-1">
        <span
          className={`w-2 h-2 rounded-full shrink-0 ${STATUS_DOT[service.status] ?? STATUS_DOT.stopped}`}
        />
        <h3 className="font-semibold text-sm truncate flex-1">
          {service.name}
        </h3>
      </div>

      {/* Row 2: Description */}
      {service.description && (
        <p className="text-xs text-text-muted mb-2 truncate pl-[18px]">
          {service.description}
        </p>
      )}

      {/* Row 3: Compact metadata */}
      <div className="text-[11px] text-text-tertiary pl-[18px] mb-3">
        <span>{service.type}</span>
        {service.pid && (
          <>
            <span className="mx-1.5">·</span>
            <span>PID {service.pid}</span>
          </>
        )}
      </div>

      {/* Row 4: Metrics with sparklines (running) or status text (stopped) */}
      <div className="pl-[18px] mb-3 min-h-[24px]">
        {isRunning && cpuHistory.length >= 2 ? (
          <div className="flex items-center gap-5">
            <div className="flex items-center gap-1.5">
              <Sparkline
                data={cpuHistory}
                width={60}
                height={20}
                color="#8b8b93"
                showValue={false}
              />
              <span className="text-[11px] text-text-muted tabular-nums">
                {service.metrics
                  ? `${service.metrics.cpu_percent.toFixed(0)}%`
                  : "—"}
              </span>
            </div>
            <div className="flex items-center gap-1.5">
              <Sparkline
                data={memHistory}
                width={60}
                height={20}
                color="#6b6b73"
                showValue={false}
              />
              <span className="text-[11px] text-text-muted tabular-nums">
                {service.metrics
                  ? `${service.metrics.memory_mb.toFixed(0)} MB`
                  : "—"}
              </span>
            </div>
          </div>
        ) : (
          <span className="text-[11px] text-text-tertiary">
            {service.status === "crashed" ? "Crashed" : "Stopped"}
          </span>
        )}
      </div>

      {/* Row 5: Actions — visible on hover only */}
      <div className="flex items-center gap-1 pl-[14px] opacity-0 group-hover:opacity-100 transition-opacity">
        {!isRunning ? (
          <ActionBtn
            icon={Play}
            title="Start"
            onClick={(e) => handleAction(e, () => start(service.id))}
          />
        ) : (
          <>
            <ActionBtn
              icon={Square}
              title="Stop"
              onClick={(e) => handleAction(e, () => stop(service.id))}
            />
            <ActionBtn
              icon={RotateCw}
              title="Restart"
              onClick={(e) => handleAction(e, () => restart(service.id))}
            />
          </>
        )}
        <ActionBtn
          icon={Terminal}
          title="Logs"
          onClick={(e) => {
            e.stopPropagation();
            navigate(`/services/${service.id}`);
          }}
        />
        {service.url && (
          <ActionBtn
            icon={ExternalLink}
            title="Open in browser"
            onClick={(e) => {
              e.stopPropagation();
              window.open(service.url!, "_blank", "noopener,noreferrer");
            }}
          />
        )}
        <div className="flex-1" />
        <ActionBtn
          icon={Trash2}
          title="Delete"
          onClick={(e) => {
            e.stopPropagation();
            if (confirm(`Delete service "${service.name}"?`)) {
              remove(service.id);
            }
          }}
        />
      </div>
    </div>
  );
}

function ActionBtn({
  icon: Icon,
  title,
  onClick,
}: {
  icon: React.ElementType;
  title: string;
  onClick: (e: React.MouseEvent) => void;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors"
    >
      <Icon className="w-3.5 h-3.5" />
    </button>
  );
}
