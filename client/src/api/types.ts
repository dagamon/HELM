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
  sort_order: number;
  stack_id: number | null;
  /** Panel color key from the active theme's `panels` palette ("" = default) */
  card_color: string | null;
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
  /** 0 detaches the service from its stack */
  stack_id?: number | null;
  card_color?: string | null;
}

export type ServiceUpdate = Partial<ServiceCreate>;

export interface Stack {
  id: number;
  name: string;
  description: string | null;
  tags: string[] | null;
  /** Panel color key from the active theme's `panels` palette ("" = default) */
  card_color: string | null;
  created_at: string;
  updated_at: string;
  service_count: number;
  running_count: number;
  status: "running" | "partial" | "stopped";
}

export interface StackCreate {
  name: string;
  description?: string | null;
  tags?: string[] | null;
  card_color?: string | null;
}

/** One entry from themes/*.json, served by GET /api/themes. */
export interface Theme {
  name: string;
  label: string;
  hint?: string;
  colors: Record<string, string>;
  /** Allowed card background tints while this theme is active. */
  panels?: Record<string, string>;
}

export type StackUpdate = Partial<StackCreate>;

export interface StackMemberOutcome {
  id: number;
  name: string;
  outcome: string;
  error?: string;
}

export interface StackActionResult {
  status: "ok" | "partial";
  services: StackMemberOutcome[];
}

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
  /** HELM release version, e.g. "0.2.0" */
  version: string;
  python_version: string;
  platform: string;
  uptime_seconds: number;
  service_count: number;
  running_count: number;
}

export interface CpuCore {
  name: string;
  usage: number;
}

export interface DiskInfo {
  name: string;
  mount: string;
  fs: string;
  total_bytes: number;
  available_bytes: number;
}

export interface HostSnapshot {
  cpu_brand: string;
  cpu_usage: number;
  core_count: number;
  cores: CpuCore[];
  mem_total_bytes: number;
  mem_used_bytes: number;
  mem_available_bytes: number;
  swap_total_bytes: number;
  swap_used_bytes: number;
  uptime_seconds: number;
  load_avg: [number, number, number];
  disks: DiskInfo[];
  ts_secs: number;
}

export interface ProcDiag {
  entity_type: string;
  entity_id: number;
  name: string;
  pid: number;
  cpu_percent: number;
  memory_mb: number;
}

export interface HelmDiag {
  uptime_seconds: number;
  service_count: number;
  running_count: number;
  cpu_percent: number;
  memory_mb: number;
}

export interface Diagnostics {
  host: HostSnapshot;
  helm: HelmDiag;
  processes: ProcDiag[];
}

export interface UpdateStatus {
  branch: string;
  current: string;
  current_short: string;
  latest: string;
  latest_short: string;
  behind: number;
  ahead: number;
  update_available: boolean;
  latest_subject: string;
  checked_at: number;
}

export interface ApplyResult {
  status: string;
  script: string;
  log: string;
}
