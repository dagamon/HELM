import { useEffect, useState } from "react";
import { Plus, Play, Pencil, Trash2, Terminal, Clock } from "lucide-react";
import { useScripts } from "@/store/scripts";
import { ScriptModal } from "@/components/modals/ScriptModal";
import { ScriptLogsModal } from "@/components/modals/ScriptLogsModal";
import { AutomationModal } from "@/components/modals/AutomationModal";

export function Scripts() {
  const { scripts, loading, fetch, run, remove } = useScripts();
  const [showAdd, setShowAdd] = useState(false);
  const [showAutomation, setShowAutomation] = useState(false);
  const [editId, setEditId] = useState<number | null>(null);
  const [logsScriptId, setLogsScriptId] = useState<number | null>(null);
  const [runningIds, setRunningIds] = useState<Set<number>>(new Set());

  useEffect(() => {
    fetch();
  }, [fetch]);

  const handleRun = async (id: number) => {
    setRunningIds((s) => new Set(s).add(id));
    try {
      await run(id);
      setLogsScriptId(id);
    } catch (err) {
      console.error(err);
    } finally {
      setRunningIds((s) => {
        const next = new Set(s);
        next.delete(id);
        return next;
      });
    }
  };

  const editScript = editId != null ? scripts.find((s) => s.id === editId) : undefined;

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-end gap-2 mb-6">
        <button
          onClick={() => setShowAutomation(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
        >
          <Clock className="w-3.5 h-3.5" />
          Automation
        </button>
        <button
          onClick={() => setShowAdd(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-lg text-text-muted hover:text-text hover:border-border-hover transition-colors"
        >
          <Plus className="w-3.5 h-3.5" />
          Add Script
        </button>
      </div>

      {loading && scripts.length === 0 ? (
        <div className="text-text-muted text-sm">Loading...</div>
      ) : scripts.length === 0 ? (
        <div className="text-center py-20 text-text-muted">
          <p className="text-lg mb-2">No scripts yet</p>
          <p className="text-sm">Click "Add Script" to create one</p>
        </div>
      ) : (
        <div className="bg-surface border border-border rounded-xl overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-text-tertiary text-xs">
                <th className="px-4 py-3 font-medium">Name</th>
                <th className="px-4 py-3 font-medium">Command</th>
                <th className="px-4 py-3 font-medium">Platform</th>
                <th className="px-4 py-3 font-medium">Schedule</th>
                <th className="px-4 py-3 font-medium w-40">Actions</th>
              </tr>
            </thead>
            <tbody>
              {scripts.map((script) => (
                <tr
                  key={script.id}
                  className="border-b border-border last:border-0 hover:bg-surface-hover transition-colors"
                >
                  <td className="px-4 py-3">
                    <div className="font-medium">{script.name}</div>
                    {script.description && (
                      <div className="text-xs text-text-muted mt-0.5">
                        {script.description}
                      </div>
                    )}
                  </td>
                  <td className="px-4 py-3 text-text-muted font-mono text-xs truncate max-w-[300px]">
                    {script.command}
                  </td>
                  <td className="px-4 py-3 text-text-muted">{script.platform}</td>
                  <td className="px-4 py-3 font-mono text-xs">
                    {script.cron_enabled && script.cron_schedule ? (
                      <span className="flex items-center gap-1.5 text-text-muted">
                        <span className="w-1.5 h-1.5 rounded-full bg-green-500 shrink-0" />
                        {script.cron_schedule}
                      </span>
                    ) : (
                      <span className="text-text-tertiary">—</span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => handleRun(script.id)}
                        disabled={runningIds.has(script.id)}
                        className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors disabled:opacity-50"
                        title="Run"
                      >
                        <Play className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={() => setLogsScriptId(script.id)}
                        className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors"
                        title="Logs"
                      >
                        <Terminal className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={() => setEditId(script.id)}
                        className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors"
                        title="Edit"
                      >
                        <Pencil className="w-3.5 h-3.5" />
                      </button>
                      <button
                        onClick={() => {
                          if (confirm(`Delete "${script.name}"?`)) remove(script.id);
                        }}
                        className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors"
                        title="Delete"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {showAdd && <ScriptModal onClose={() => setShowAdd(false)} />}
      {editScript && (
        <ScriptModal script={editScript} onClose={() => setEditId(null)} />
      )}
      {logsScriptId != null && (
        <ScriptLogsModal
          scriptId={logsScriptId}
          onClose={() => setLogsScriptId(null)}
        />
      )}
      {showAutomation && <AutomationModal onClose={() => setShowAutomation(false)} />}
    </div>
  );
}
