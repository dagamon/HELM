export interface Service {
  id: number;
  name: string;
  description: string | null;
  type: string;
  command: string | null;
  cwd: string | null;
  venv_path: string | null;
  args: string[] | null;
  env: Record<string, string> | null;
  url: string | null;
  health_check_url: string | null;
  health_check_interval: number;
  auto_start: boolean;
  restart_on_crash: boolean;
  platform: string;
  tags: string[] | null;
  depends_on: number[] | null;
  webhook_url: string | null;
  manifest_path: string | null;
  binary_path: string | null;
  cargo_profile: string | null;
  cargo_features: string | null;
  prebuild: boolean;
  status: string;
  pid: number | null;
  metrics: MetricsSnapshot | null;
  created_at: string;
  updated_at: string;
}

export interface ServiceCreate {
  name: string;
  description?: string | null;
  type: string;
  command?: string | null;
  cwd?: string | null;
  venv_path?: string | null;
  args?: string[] | null;
  env?: Record<string, string> | null;
  url?: string | null;
  health_check_url?: string | null;
  health_check_interval?: number;
  auto_start?: boolean;
  restart_on_crash?: boolean;
  platform?: string;
  tags?: string[] | null;
  depends_on?: number[] | null;
  webhook_url?: string | null;
  manifest_path?: string | null;
  binary_path?: string | null;
  cargo_profile?: string | null;
  cargo_features?: string | null;
  prebuild?: boolean;
}

export type ServiceUpdate = Partial<ServiceCreate>;

export interface Script {
  id: number;
  name: string;
  description: string | null;
  command: string;
  run_mode: "exec" | "shell";
  cwd: string | null;
  args: string[] | null;
  platform: string;
  tags: string[] | null;
  cron_schedule: string | null;
  cron_enabled: boolean;
  created_at: string;
}

export interface ScriptCreate {
  name: string;
  description?: string | null;
  command: string;
  run_mode?: "exec" | "shell";
  cwd?: string | null;
  args?: string[] | null;
  platform?: string;
  tags?: string[] | null;
  cron_schedule?: string | null;
  cron_enabled?: boolean;
}

export type ScriptUpdate = Partial<ScriptCreate>;

export type SchedulerNextRuns = Record<string, string>;

export interface RunLog {
  id: number;
  entity_type: string;
  entity_id: number;
  started_at: string;
  stopped_at: string | null;
  exit_code: number | null;
  status: string;
  pid: number | null;
}

export interface LogLine {
  stream: string;
  line?: string;
  text?: string;
  ts: string;
}

export interface MetricsSnapshot {
  cpu_percent: number;
  memory_mb: number;
  ts: string;
}

export interface StatusEvent {
  entity_type: string;
  entity_id: number;
  status: string;
  pid: number | null;
  metrics: MetricsSnapshot | null;
}

export interface SystemInfo {
  os: string;
  python_version: string;
  platform: string;
  uptime_seconds: number;
  service_count: number;
  running_count: number;
}
