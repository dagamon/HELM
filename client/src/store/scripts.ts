import { create } from "zustand";
import { api } from "@/api/client";
import type { Script, ScriptCreate, ScriptUpdate, RunLog, SchedulerNextRuns } from "@/api/types";

interface ScriptsState {
  scripts: Script[];
  runs: Map<number, RunLog>;
  nextRuns: SchedulerNextRuns;
  loading: boolean;
  error: string | null;

  fetch: () => Promise<void>;
  create: (data: ScriptCreate) => Promise<Script>;
  update: (id: number, data: ScriptUpdate) => Promise<void>;
  remove: (id: number) => Promise<void>;
  run: (id: number) => Promise<number>;
  pollRun: (runId: number) => Promise<void>;
  fetchNextRuns: () => Promise<void>;
}

export const useScripts = create<ScriptsState>((set, get) => ({
  scripts: [],
  runs: new Map(),
  nextRuns: {},
  loading: false,
  error: null,

  fetch: async () => {
    set({ loading: true, error: null });
    try {
      const scripts = await api.listScripts();
      set({ scripts, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  create: async (data) => {
    const script = await api.createScript(data);
    set((s) => ({ scripts: [...s.scripts, script] }));
    return script;
  },

  update: async (id, data) => {
    const script = await api.updateScript(id, data);
    set((s) => ({ scripts: s.scripts.map((x) => (x.id === id ? script : x)) }));
  },

  remove: async (id) => {
    await api.deleteScript(id);
    set((s) => ({ scripts: s.scripts.filter((x) => x.id !== id) }));
  },

  run: async (id) => {
    const { run_id } = await api.runScript(id);
    get().pollRun(run_id);
    return run_id;
  },

  pollRun: async (runId) => {
    const poll = async () => {
      const log = await api.getRunStatus(runId);
      set((s) => {
        const runs = new Map(s.runs);
        runs.set(runId, log);
        return { runs };
      });
      if (log.status === "running") {
        setTimeout(poll, 2000);
      }
    };
    poll();
  },

  fetchNextRuns: async () => {
    try {
      const nextRuns = await api.getSchedulerNextRuns();
      set({ nextRuns });
    } catch {
      // non-critical
    }
  },
}));
