import { create } from "zustand";
import { api } from "@/api/client";
import type { Stack, StackCreate, StackUpdate } from "@/api/types";
import { useServices } from "@/store/services";

interface StacksState {
  stacks: Stack[];
  loading: boolean;
  error: string | null;

  fetch: () => Promise<void>;
  create: (data: StackCreate) => Promise<Stack>;
  update: (id: number, data: StackUpdate) => Promise<void>;
  remove: (id: number) => Promise<void>;
  start: (id: number) => Promise<void>;
  stop: (id: number) => Promise<void>;
  restart: (id: number) => Promise<void>;
}

// Stack actions change member service states in bulk; re-fetch services so
// cards update immediately instead of waiting for the next WS status frame.
async function refreshServices() {
  await useServices.getState().fetch();
}

export const useStacks = create<StacksState>((set) => ({
  stacks: [],
  loading: false,
  error: null,

  fetch: async () => {
    set({ loading: true, error: null });
    try {
      const stacks = await api.listStacks();
      set({ stacks, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  create: async (data) => {
    const stack = await api.createStack(data);
    set((s) => ({ stacks: [...s.stacks, stack] }));
    return stack;
  },

  update: async (id, data) => {
    const stack = await api.updateStack(id, data);
    set((s) => ({ stacks: s.stacks.map((x) => (x.id === id ? stack : x)) }));
  },

  remove: async (id) => {
    await api.deleteStack(id);
    set((s) => ({ stacks: s.stacks.filter((x) => x.id !== id) }));
    await refreshServices(); // members got detached server-side
  },

  start: async (id) => {
    await api.startStack(id);
    await refreshServices();
  },

  stop: async (id) => {
    await api.stopStack(id);
    await refreshServices();
  },

  restart: async (id) => {
    await api.restartStack(id);
    await refreshServices();
  },
}));
