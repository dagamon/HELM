import type {
  Service,
  ServiceCreate,
  ServiceUpdate,
  Script,
  ScriptCreate,
  ScriptUpdate,
  RunLog,
  LogLine,
  SystemInfo,
  SchedulerNextRuns,
} from "./types";

const BASE = "/api";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...init,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`${res.status}: ${body}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

// Services
export const api = {
  // System
  health: () => request<{ status: string }>("/health"),
  systemInfo: () => request<SystemInfo>("/system/info"),

  // Services CRUD
  listServices: () => request<Service[]>("/services"),
  getService: (id: number) => request<Service>(`/services/${id}`),
  createService: (data: ServiceCreate) =>
    request<Service>("/services", { method: "POST", body: JSON.stringify(data) }),
  updateService: (id: number, data: ServiceUpdate) =>
    request<Service>(`/services/${id}`, { method: "PUT", body: JSON.stringify(data) }),
  deleteService: (id: number) =>
    request<void>(`/services/${id}`, { method: "DELETE" }),

  // Service actions
  startService: (id: number) =>
    request<{ status: string; pid: number }>(`/services/${id}/start`, { method: "POST" }),
  stopService: (id: number) =>
    request<{ status: string }>(`/services/${id}/stop`, { method: "POST" }),
  restartService: (id: number) =>
    request<{ status: string; pid: number }>(`/services/${id}/restart`, { method: "POST" }),
  getServiceLogs: (id: number, limit = 200) =>
    request<LogLine[]>(`/services/${id}/logs?limit=${limit}`),

  // Scripts CRUD
  listScripts: () => request<Script[]>("/scripts"),
  createScript: (data: ScriptCreate) =>
    request<Script>("/scripts", { method: "POST", body: JSON.stringify(data) }),
  updateScript: (id: number, data: ScriptUpdate) =>
    request<Script>(`/scripts/${id}`, { method: "PUT", body: JSON.stringify(data) }),
  deleteScript: (id: number) =>
    request<void>(`/scripts/${id}`, { method: "DELETE" }),

  // Script actions
  runScript: (id: number) =>
    request<{ run_id: number; status: string }>(`/scripts/${id}/run`, { method: "POST" }),
  getRunStatus: (runId: number) =>
    request<RunLog>(`/scripts/runs/${runId}`),
  getSchedulerNextRuns: () =>
    request<SchedulerNextRuns>("/scripts/scheduler/next-run"),

  // FAQ
  listFaqArticles: () =>
    request<{ slug: string; title: string }[]>("/faq/articles"),
  getFaqArticle: (slug: string) =>
    request<{ slug: string; title: string; content: string }>(`/faq/articles/${slug}`),
};
