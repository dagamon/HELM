import { create } from "zustand";
import { api } from "@/api/client";
import type { Service, ServiceCreate, ServiceUpdate } from "@/api/types";

interface ServicesState {
  services: Service[];
  loading: boolean;
  error: string | null;

  fetch: () => Promise<void>;
  create: (data: ServiceCreate) => Promise<Service>;
  update: (id: number, data: ServiceUpdate) => Promise<void>;
  remove: (id: number) => Promise<void>;
  start: (id: number) => Promise<void>;
  stop: (id: number) => Promise<void>;
  restart: (id: number) => Promise<void>;
  patchLocal: (id: number, patch: Partial<Service>) => void;
}

export const useServices = create<ServicesState>((set, get) => ({
  services: [],
  loading: false,
  error: null,

  fetch: async () => {
    set({ loading: true, error: null });
    try {
      const services = await api.listServices();
      set({ services, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  create: async (data) => {
    const svc = await api.createService(data);
    set((s) => ({ services: [...s.services, svc] }));
    return svc;
  },

  update: async (id, data) => {
    const svc = await api.updateService(id, data);
    set((s) => ({ services: s.services.map((x) => (x.id === id ? svc : x)) }));
  },

  remove: async (id) => {
    await api.deleteService(id);
    set((s) => ({ services: s.services.filter((x) => x.id !== id) }));
  },

  start: async (id) => {
    const { pid } = await api.startService(id);
    get().patchLocal(id, { status: "running", pid });
  },

  stop: async (id) => {
    await api.stopService(id);
    get().patchLocal(id, { status: "stopped", pid: null });
  },

  restart: async (id) => {
    const { pid } = await api.restartService(id);
    get().patchLocal(id, { status: "running", pid });
  },

  patchLocal: (id, patch) => {
    set((s) => ({
      services: s.services.map((x) => (x.id === id ? { ...x, ...patch } : x)),
    }));
  },
}));
