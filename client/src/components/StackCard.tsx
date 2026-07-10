import { useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  Folder,
  FolderOpen,
  Play,
  Square,
  RotateCw,
  Pencil,
  Trash2,
} from "lucide-react";
import type { Stack } from "@/api/types";
import { useServices } from "@/store/services";
import { useStacks } from "@/store/stacks";
import { useThemeStore } from "@/store/theme";
import { PanelColorPicker } from "./PanelColorPicker";

const STATUS_DOT: Record<string, string> = {
  running: "bg-success",
  partial: "bg-warning",
  stopped: "bg-text-tertiary",
};

export function StackCard({
  stack,
  onEdit,
  dropActive,
  onDropService,
}: {
  stack: Stack;
  onEdit: () => void;
  /** True while a service card is being dragged on the dashboard. */
  dropActive?: boolean;
  /** Called with the dragged service id when it is dropped onto this stack. */
  onDropService?: (serviceId: number) => void;
}) {
  const navigate = useNavigate();
  const { services } = useServices();
  const { start, stop, restart, remove, update } = useStacks();
  const [busy, setBusy] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const panels = useThemeStore((s) => s.panels);
  const panelHex = stack.card_color ? panels[stack.card_color] : undefined;

  // Live membership/status from the services store (WS keeps it fresh),
  // not the snapshot counts baked into the stack response.
  const members = services.filter((s) => s.stack_id === stack.id);
  const running = members.filter((s) => s.status === "running").length;
  const status =
    members.length > 0 && running === members.length
      ? "running"
      : running > 0
        ? "partial"
        : "stopped";

  const act = async (e: React.MouseEvent, action: () => Promise<void>) => {
    e.stopPropagation();
    setBusy(true);
    try {
      await action();
    } catch (err) {
      console.error(err);
    } finally {
      setBusy(false);
    }
  };

  const droppable = dropActive && onDropService;

  return (
    <div
      onClick={() => navigate(`/stacks/${stack.id}`)}
      onDragOver={
        droppable
          ? (e) => {
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
            }
          : undefined
      }
      onDragEnter={droppable ? () => setDragOver(true) : undefined}
      onDragLeave={
        droppable
          ? (e) => {
              if (!e.currentTarget.contains(e.relatedTarget as Node))
                setDragOver(false);
            }
          : undefined
      }
      onDrop={
        droppable
          ? (e) => {
              e.preventDefault();
              setDragOver(false);
              const id = Number(e.dataTransfer.getData("text/plain"));
              if (id) onDropService(id);
            }
          : undefined
      }
      style={panelHex ? { background: panelHex } : undefined}
      className={`group bg-surface border rounded-xl p-4 hover:bg-surface-hover hover:border-border-hover transition-colors cursor-pointer ${
        dragOver ? "border-accent ring-1 ring-accent" : "border-border"
      }`}
    >
      {/* Row 1: folder icon + name + status dot */}
      <div className="flex items-center gap-2.5 mb-1">
        <span className="relative shrink-0 text-text-muted">
          <Folder className="w-4 h-4 group-hover:hidden" />
          <FolderOpen className="w-4 h-4 hidden group-hover:block" />
        </span>
        <h3 className="font-semibold text-sm truncate flex-1">{stack.name}</h3>
        <span
          className={`w-2 h-2 rounded-full shrink-0 ${STATUS_DOT[status]}`}
        />
      </div>

      {/* Row 2: description */}
      {stack.description && (
        <p className="text-xs text-text-muted mb-2 truncate pl-[26px]">
          {stack.description}
        </p>
      )}

      {/* Row 3: membership summary */}
      <div className="text-[11px] text-text-tertiary pl-[26px] mb-3">
        <span>
          {members.length === 1 ? "1 service" : `${members.length} services`}
        </span>
        {members.length > 0 && (
          <>
            <span className="mx-1.5">·</span>
            <span className={running > 0 ? "text-text-muted" : ""}>
              {running} running
            </span>
          </>
        )}
        {stack.tags && stack.tags.length > 0 && (
          <>
            <span className="mx-1.5">·</span>
            <span>{stack.tags.join(", ")}</span>
          </>
        )}
      </div>

      {/* Row 4: actions — visible on hover */}
      <div className="flex items-center gap-1 pl-[22px] opacity-0 group-hover:opacity-100 transition-opacity">
        {status !== "running" && (
          <ActionBtn
            icon={Play}
            title="Start all"
            disabled={busy || members.length === 0}
            onClick={(e) => act(e, () => start(stack.id))}
          />
        )}
        {status !== "stopped" && (
          <>
            <ActionBtn
              icon={Square}
              title="Stop all"
              disabled={busy}
              onClick={(e) => act(e, () => stop(stack.id))}
            />
            <ActionBtn
              icon={RotateCw}
              title="Restart all"
              disabled={busy}
              spinning={busy}
              onClick={(e) => act(e, () => restart(stack.id))}
            />
          </>
        )}
        <PanelColorPicker
          value={stack.card_color}
          onChange={(key) => update(stack.id, { card_color: key })}
        />
        <div className="flex-1" />
        <ActionBtn
          icon={Pencil}
          title="Edit stack"
          onClick={(e) => {
            e.stopPropagation();
            onEdit();
          }}
        />
        <ActionBtn
          icon={Trash2}
          title="Delete stack (services survive)"
          onClick={(e) => {
            e.stopPropagation();
            if (
              confirm(
                `Delete stack "${stack.name}"? Its services stay, ungrouped.`,
              )
            ) {
              remove(stack.id);
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
  disabled,
  spinning,
}: {
  icon: React.ElementType;
  title: string;
  onClick: (e: React.MouseEvent) => void;
  disabled?: boolean;
  spinning?: boolean;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      disabled={disabled}
      className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors disabled:opacity-40 disabled:pointer-events-none"
    >
      <Icon className={`w-3.5 h-3.5 ${spinning ? "animate-spin" : ""}`} />
    </button>
  );
}
