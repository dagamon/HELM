import { useEffect, useState } from "react";
import { Clock, Loader2 } from "lucide-react";
import { useScripts } from "@/store/scripts";
import { Modal } from "./Modal";
import { Checkbox } from "./FormField";
import { CronBuilder } from "./CronBuilder";

interface Props {
  onClose: () => void;
}

function formatNextRun(iso: string | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  const diffMs = d.getTime() - Date.now();
  if (diffMs < 0) return "overdue";
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "< 1m";
  if (diffMin < 60) return `in ${diffMin}m`;
  const h = Math.floor(diffMin / 60);
  const m = diffMin % 60;
  if (h < 24) return `in ${h}h ${m}m`;
  return d.toLocaleString();
}

export function AutomationModal({ onClose }: Props) {
  const { scripts, update, fetchNextRuns, nextRuns } = useScripts();
  const [saving, setSaving] = useState<Record<number, boolean>>({});
  const [localSchedules, setLocalSchedules] = useState<Record<number, string>>(() => {
    const init: Record<number, string> = {};
    scripts.forEach((s) => {
      init[s.id] = s.cron_schedule ?? "";
    });
    return init;
  });

  useEffect(() => {
    fetchNextRuns();
    const timer = setInterval(fetchNextRuns, 30_000);
    return () => clearInterval(timer);
  }, [fetchNextRuns]);

  const handleToggle = async (id: number, enabled: boolean) => {
    setSaving((s) => ({ ...s, [id]: true }));
    try {
      await update(id, { cron_enabled: enabled });
      await fetchNextRuns();
    } finally {
      setSaving((s) => ({ ...s, [id]: false }));
    }
  };

  const handleScheduleSave = async (id: number, value: string) => {
    const script = scripts.find((s) => s.id === id);
    const prev = script?.cron_schedule ?? "";
    if (value === prev) return;
    setSaving((s) => ({ ...s, [id]: true }));
    try {
      await update(id, { cron_schedule: value || null });
      await fetchNextRuns();
    } finally {
      setSaving((s) => ({ ...s, [id]: false }));
    }
  };

  return (
    <Modal title="Automation" onClose={onClose} wide>
      {scripts.length === 0 ? (
        <p className="text-sm text-text-muted text-center py-8">No scripts available</p>
      ) : (
        <div className="space-y-2">
          {scripts.map((script) => {
            const nextRun = nextRuns[String(script.id)];
            return (
              <div
                key={script.id}
                className="flex items-start gap-3 px-4 py-3 rounded-lg border border-border hover:bg-surface-hover transition-colors"
              >
                <div className="flex-1 min-w-0 pt-1">
                  <div className="font-medium text-sm truncate">{script.name}</div>
                  {script.description && (
                    <div className="text-xs text-text-muted truncate">{script.description}</div>
                  )}
                </div>

                <div className="w-52 shrink-0">
                  <CronBuilder
                    compact
                    value={localSchedules[script.id] ?? ""}
                    onChange={(v) =>
                      setLocalSchedules((s) => ({ ...s, [script.id]: v }))
                    }
                    onCommit={(v) => handleScheduleSave(script.id, v)}
                  />
                </div>

                <div className="flex items-center gap-1.5 w-24 shrink-0 text-xs text-text-muted pt-2">
                  <Clock className="w-3 h-3 shrink-0" />
                  <span title={nextRun} className="truncate">
                    {script.cron_enabled ? formatNextRun(nextRun) : "—"}
                  </span>
                </div>

                <div className="shrink-0 pt-2">
                  <Checkbox
                    label="Enable"
                    checked={script.cron_enabled}
                    onChange={(v) => handleToggle(script.id, v)}
                  />
                </div>

                <div className="w-4 shrink-0 pt-2">
                  {saving[script.id] && (
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-text-tertiary" />
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Modal>
  );
}
