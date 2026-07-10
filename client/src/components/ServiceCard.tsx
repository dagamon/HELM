import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Play,
  Square,
  RotateCw,
  Trash2,
  Terminal,
  ExternalLink,
  GripVertical,
} from "lucide-react";
import type { Service } from "@/api/types";
import { useServices } from "@/store/services";
import { useThemeStore } from "@/store/theme";
import { Sparkline } from "./Sparkline";
import { PanelColorPicker } from "./PanelColorPicker";

export interface ServiceCardDrag {
  onDragStart: () => void;
  onDragEnter: () => void;
  onDrop: () => void;
  onDragEnd: () => void;
  dragging: boolean;
  dropTarget: boolean;
}

const MAX_HISTORY = 30;

const STATUS_DOT: Record<string, string> = {
  running: "bg-success",
  stopped: "bg-text-tertiary",
  crashed: "bg-danger",
  error: "bg-danger",
};

export function ServiceCard({
  service,
  drag,
}: {
  service: Service;
  drag?: ServiceCardDrag;
}) {
  const { start, stop, restart, remove, update } = useServices();
  const navigate = useNavigate();
  const isRunning = service.status === "running";
  const panels = useThemeStore((s) => s.panels);
  const panelHex = service.card_color ? panels[service.card_color] : undefined;

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
      draggable={drag ? true : undefined}
      onDragStart={
        drag
          ? (e) => {
              e.dataTransfer.effectAllowed = "move";
              e.dataTransfer.setData("text/plain", String(service.id));
              drag.onDragStart();
            }
          : undefined
      }
      onDragEnter={drag ? drag.onDragEnter : undefined}
      onDragOver={drag ? (e) => e.preventDefault() : undefined}
      onDrop={
        drag
          ? (e) => {
              e.preventDefault();
              drag.onDrop();
            }
          : undefined
      }
      onDragEnd={drag ? drag.onDragEnd : undefined}
      style={panelHex ? { background: panelHex } : undefined}
      className={`group bg-surface border rounded-xl p-4 hover:bg-surface-hover hover:border-border-hover transition-all cursor-pointer ${
        drag?.dropTarget
          ? "border-accent ring-1 ring-accent"
          : "border-border"
      } ${drag?.dragging ? "opacity-40" : ""}`}
    >
      {/* Row 1: Status dot + Name */}
      <div className="flex items-center gap-2.5 mb-1">
        {drag && (
          <GripVertical
            className="w-3.5 h-3.5 -ml-1 shrink-0 text-text-tertiary opacity-0 group-hover:opacity-100 transition-opacity cursor-grab active:cursor-grabbing"
            onClick={(e) => e.stopPropagation()}
          />
        )}
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
        <PanelColorPicker
          value={service.card_color}
          onChange={(key) => update(service.id, { card_color: key })}
        />
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
