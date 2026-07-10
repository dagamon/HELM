import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { motion } from "framer-motion";
import {
  ArrowLeft,
  ChevronDown,
  FolderOpen,
  Link2,
  Pencil,
  Play,
  Plus,
  RotateCw,
  Square,
  Trash2,
} from "lucide-react";
import { useServices } from "@/store/services";
import { useStacks } from "@/store/stacks";
import { ServiceCard } from "@/components/ServiceCard";
import { ServiceModal } from "@/components/modals/ServiceModal";
import { StackModal } from "@/components/modals/StackModal";

const STATUS_DOT: Record<string, string> = {
  running: "bg-success",
  partial: "bg-warning",
  stopped: "bg-text-tertiary",
};

export function StackDetail() {
  const { id } = useParams();
  const stackId = Number(id);
  const navigate = useNavigate();

  const { services, fetch, update, moveService, persistOrder } = useServices();
  const {
    stacks,
    fetch: fetchStacks,
    start,
    stop,
    restart,
    remove,
  } = useStacks();

  const [showAdd, setShowAdd] = useState(false);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [dragId, setDragId] = useState<number | null>(null);

  useEffect(() => {
    if (services.length === 0) fetch();
    if (stacks.length === 0) fetchStacks();
  }, [services.length, stacks.length, fetch, fetchStacks]);

  const stack = stacks.find((s) => s.id === stackId);
  const members = useMemo(
    () => services.filter((s) => s.stack_id === stackId),
    [services, stackId],
  );
  const ungrouped = useMemo(
    () => services.filter((s) => s.stack_id == null),
    [services],
  );
  const running = members.filter((s) => s.status === "running").length;
  const status =
    members.length > 0 && running === members.length
      ? "running"
      : running > 0
        ? "partial"
        : "stopped";

  const act = async (action: () => Promise<void>) => {
    setBusy(true);
    try {
      await action();
    } catch (err) {
      console.error(err);
    } finally {
      setBusy(false);
    }
  };

  if (!stack) {
    return (
      <div className="max-w-6xl mx-auto text-text-muted text-sm">
        {stacks.length === 0 ? "Loading..." : "Stack not found"}
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto">
      {/* Header */}
      <div className="mb-6">
        <button
          onClick={() => navigate("/")}
          className="flex items-center gap-1.5 text-xs text-text-tertiary hover:text-text-muted transition-colors mb-4"
        >
          <ArrowLeft className="w-3.5 h-3.5" />
          Dashboard
        </button>

        <div className="flex flex-wrap items-center gap-3">
          <FolderOpen className="w-5 h-5 text-text-muted shrink-0" />
          <h1 className="text-lg font-semibold">{stack.name}</h1>
          <span className={`w-2 h-2 rounded-full ${STATUS_DOT[status]}`} />
          <span className="text-xs text-text-tertiary">
            {members.length === 1 ? "1 service" : `${members.length} services`}
            {members.length > 0 && <> · {running} running</>}
          </span>

          <div className="flex-1" />

          {/* Stack actions */}
          <div className="flex items-center gap-2">
            {status !== "running" && (
              <HeaderBtn
                icon={Play}
                label="Start"
                disabled={busy || members.length === 0}
                onClick={() => act(() => start(stack.id))}
              />
            )}
            {status !== "stopped" && (
              <>
                <HeaderBtn
                  icon={Square}
                  label="Stop"
                  disabled={busy}
                  onClick={() => act(() => stop(stack.id))}
                />
                <HeaderBtn
                  icon={RotateCw}
                  label="Restart"
                  disabled={busy}
                  spinning={busy}
                  onClick={() => act(() => restart(stack.id))}
                />
              </>
            )}
            <HeaderBtn
              icon={Pencil}
              label="Edit"
              onClick={() => setEditing(true)}
            />
            <HeaderBtn
              icon={Trash2}
              label="Delete"
              onClick={() => {
                if (
                  confirm(
                    `Delete stack "${stack.name}"? Its services stay, ungrouped.`,
                  )
                ) {
                  remove(stack.id).then(() => navigate("/"));
                }
              }}
            />
          </div>
        </div>

        {stack.description && (
          <p className="text-sm text-text-muted mt-2 max-w-[75ch]">
            {stack.description}
          </p>
        )}
        {stack.tags && stack.tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5 mt-2.5">
            {stack.tags.map((tag) => (
              <span
                key={tag}
                className="px-2 py-0.5 text-[11px] rounded-md bg-surface border border-border text-text-muted"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Toolbar */}
      <div className="flex items-center gap-2.5 mb-5">
        <div className="flex-1" />
        <AttachDropdown
          candidates={ungrouped}
          onAttach={(serviceId) => update(serviceId, { stack_id: stackId })}
        />
        <button
          onClick={() => setShowAdd(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          Add Service
        </button>
      </div>

      {/* Members grid */}
      {members.length === 0 ? (
        <div className="text-center py-16 text-text-muted">
          <p className="text-sm mb-1">This stack has no services yet</p>
          <p className="text-xs text-text-tertiary">
            Create a new service here or attach an existing one
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-5">
          {members.map((svc) => (
            <motion.div
              key={svc.id}
              layout
              transition={{ type: "spring", stiffness: 400, damping: 32 }}
            >
              <ServiceCard
                service={svc}
                drag={{
                  onDragStart: () => setDragId(svc.id),
                  onDragEnter: () => {
                    if (dragId != null && dragId !== svc.id)
                      moveService(dragId, svc.id);
                  },
                  onDrop: () => {},
                  onDragEnd: () => {
                    setDragId(null);
                    persistOrder();
                  },
                  dragging: dragId === svc.id,
                  dropTarget: false,
                }}
              />
            </motion.div>
          ))}
        </div>
      )}

      {showAdd && (
        <ServiceModal
          defaultStackId={stackId}
          onClose={() => setShowAdd(false)}
        />
      )}
      {editing && <StackModal stack={stack} onClose={() => setEditing(false)} />}
    </div>
  );
}

function HeaderBtn({
  icon: Icon,
  label,
  onClick,
  disabled,
  spinning,
}: {
  icon: React.ElementType;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  spinning?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors disabled:opacity-40 disabled:pointer-events-none"
    >
      <Icon className={`w-3.5 h-3.5 ${spinning ? "animate-spin" : ""}`} />
      {label}
    </button>
  );
}

/// Dropdown listing ungrouped services to attach to this stack.
function AttachDropdown({
  candidates,
  onAttach,
}: {
  candidates: { id: number; name: string }[];
  onAttach: (id: number) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
      >
        <Link2 className="w-3.5 h-3.5" />
        Attach Existing
        <ChevronDown
          className={`w-3 h-3 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <div className="absolute top-full mt-1 right-0 w-56 bg-surface border border-border rounded-lg shadow-lg z-30 py-1 max-h-56 overflow-y-auto">
          {candidates.length === 0 ? (
            <div className="px-3 py-2 text-xs text-text-muted">
              No ungrouped services
            </div>
          ) : (
            candidates.map((svc) => (
              <button
                key={svc.id}
                onClick={() => {
                  onAttach(svc.id);
                  setOpen(false);
                }}
                className="w-full px-3 py-1.5 text-xs text-left text-text-muted hover:text-text hover:bg-surface-hover transition-colors truncate"
              >
                {svc.name}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
