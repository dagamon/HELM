import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowUpCircle, X, Loader2 } from "lucide-react";
import { api } from "@/api/client";
import { usePrefs } from "@/store/prefs";
import type { UpdateStatus } from "@/api/types";

// Global, low-noise update watcher. Polls the git update endpoint on the
// user-configured cadence and surfaces a corner toast when commits are waiting.
export function UpdateNotice() {
  const enabled = usePrefs((s) => s.updateCheckEnabled);
  const intervalMin = usePrefs((s) => s.updateCheckIntervalMin);
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  const poll = useCallback(async () => {
    try {
      setStatus(await api.checkUpdate());
    } catch {
      // offline / not a git checkout — stay silent
    }
  }, []);

  useEffect(() => {
    if (timer.current) clearInterval(timer.current);
    if (!enabled) return;
    poll();
    const ms = Math.max(1, intervalMin) * 60_000;
    timer.current = setInterval(poll, ms);
    return () => {
      if (timer.current) clearInterval(timer.current);
    };
  }, [enabled, intervalMin, poll]);

  const apply = async () => {
    if (!window.confirm("Pull latest, rebuild and restart HELM now? The dashboard will be briefly unavailable.")) {
      return;
    }
    setApplying(true);
    try {
      await api.applyUpdate();
    } catch (e) {
      alert(`Update failed to start: ${e instanceof Error ? e.message : e}`);
      setApplying(false);
    }
  };

  if (!status || !status.update_available) return null;
  if (dismissed === status.latest) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 w-80 rounded-xl border border-accent/40 bg-surface-raised/95 backdrop-blur-md shadow-xl p-4 space-y-2">
      <div className="flex items-start gap-2">
        <ArrowUpCircle className="w-4 h-4 text-accent shrink-0 mt-0.5" />
        <div className="flex-1">
          <p className="text-sm font-medium">Update available</p>
          <p className="text-xs text-text-muted mt-0.5">
            {status.behind} commit{status.behind === 1 ? "" : "s"} behind on{" "}
            <span className="font-mono">{status.branch}</span>
          </p>
          {status.latest_subject && (
            <p className="text-xs text-text-muted mt-1 line-clamp-2">“{status.latest_subject}”</p>
          )}
        </div>
        <button
          onClick={() => setDismissed(status.latest)}
          className="p-1 rounded-md text-text-muted hover:text-text hover:bg-surface-hover"
          title="Dismiss"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
      <button
        onClick={apply}
        disabled={applying}
        className="w-full inline-flex items-center justify-center gap-2 px-3 py-1.5 rounded-lg bg-accent hover:bg-accent-hover text-white text-sm font-medium disabled:opacity-60"
      >
        {applying ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <ArrowUpCircle className="w-3.5 h-3.5" />}
        {applying ? "Updating…" : "Update now"}
      </button>
    </div>
  );
}
