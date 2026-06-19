import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { ArrowLeft, Play, Square, RotateCw, Pencil, Trash2, ExternalLink } from "lucide-react";
import { useServices } from "@/store/services";
import { StatusBadge } from "@/components/StatusBadge";
import { LogViewer } from "@/components/LogViewer";
import { MetricsPanel } from "@/components/MetricsPanel";
import { ServiceModal } from "@/components/modals/ServiceModal";

export function ServiceDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { services, fetch, start, stop, restart, remove } = useServices();
  const [showEdit, setShowEdit] = useState(false);

  const serviceId = Number(id);
  const service = services.find((s) => s.id === serviceId);

  useEffect(() => {
    if (!service) fetch();
  }, [service, fetch]);

  if (!service) {
    return <div className="text-text-muted text-sm">Loading...</div>;
  }

  const isRunning = service.status === "running";

  return (
    <div className="max-w-6xl mx-auto w-full flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 mb-6">
        <button
          onClick={() => navigate("/")}
          className="p-1.5 rounded-md hover:bg-surface-hover text-text-tertiary hover:text-text-muted transition-colors"
        >
          <ArrowLeft className="w-4 h-4" />
        </button>
        <h1 className="text-base font-semibold">{service.name}</h1>
        <StatusBadge status={service.status} />
        <div className="flex-1" />
        <div className="flex items-center gap-0.5">
          {!isRunning ? (
            <ActionButton icon={Play} label="Start" onClick={() => start(serviceId)} />
          ) : (
            <>
              <ActionButton icon={Square} label="Stop" onClick={() => stop(serviceId)} />
              <ActionButton icon={RotateCw} label="Restart" onClick={() => restart(serviceId)} />
            </>
          )}
          {service.url && (
            <ActionButton
              icon={ExternalLink}
              label="Open"
              onClick={() => window.open(service.url!, "_blank", "noopener,noreferrer")}
            />
          )}
          <div className="w-px h-4 bg-border mx-1.5" />
          <ActionButton icon={Pencil} label="Edit" onClick={() => setShowEdit(true)} />
          <ActionButton
            icon={Trash2}
            label="Delete"
            onClick={() => {
              if (confirm(`Delete "${service.name}"?`)) {
                remove(serviceId).then(() => navigate("/"));
              }
            }}
          />
        </div>
      </div>

      {/* Info cells */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-2 mb-5">
        <InfoCell label="Type" value={service.type} />
        <InfoCell label="PID" value={service.pid ?? "—"} />
        <InfoCell label="Platform" value={service.platform} />
        <InfoCell label="Auto Start" value={service.auto_start ? "Yes" : "No"} />
        {service.command && (
          <div className="col-span-2 md:col-span-4">
            <InfoCell label="Command" value={service.command} mono />
          </div>
        )}
        {service.cwd && (
          <div className="col-span-2 md:col-span-4">
            <InfoCell label="Working Dir" value={service.cwd} mono />
          </div>
        )}
      </div>

      {/* Metrics */}
      <div className="mb-5">
        <MetricsPanel service={service} />
      </div>

      {/* Logs */}
      <div className="flex-1 min-h-0">
        <LogViewer entityType="service" entityId={serviceId} />
      </div>

      {showEdit && (
        <ServiceModal service={service} onClose={() => setShowEdit(false)} />
      )}
    </div>
  );
}

function ActionButton({
  icon: Icon,
  label,
  onClick,
}: {
  icon: React.ElementType;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg hover:bg-surface-hover text-text-tertiary hover:text-text-muted transition-colors"
    >
      <Icon className="w-3.5 h-3.5" />
      {label}
    </button>
  );
}

function InfoCell({
  label,
  value,
  mono,
}: {
  label: string;
  value: string | number;
  mono?: boolean;
}) {
  return (
    <div className="bg-surface rounded-lg px-3 py-2.5">
      <div className="text-[10px] text-text-tertiary uppercase tracking-wide mb-1">{label}</div>
      <div
        className={`text-sm truncate ${mono ? "font-mono text-xs text-text-muted" : "font-medium"}`}
      >
        {String(value)}
      </div>
    </div>
  );
}
