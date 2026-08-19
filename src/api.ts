/** Typed mirror of the `#[tauri::command]`s in `src-tauri/src/lib.rs`. */
import { invoke } from "@tauri-apps/api/core";

/** One `--lua-desync` call. Order within a strategy is significant. */
export type Action = {
  function: string;
  payload?: string[];
  args: Record<string, string>;
};

/** Profile-level `--filter-*` / `--hostlist*` / `--ipset*` options. */
export type Filter = {
  l3?: string;
  tcp?: string;
  udp?: string;
  l7?: string[];
  hostlist?: string;
  hostlist_exclude?: string;
  hostlist_auto?: string;
  ipset?: string;
  ipset_exclude?: string;
};

/** One nfqws2 profile. */
export type Strategy = {
  name: string;
  enabled: boolean;
  filter: Filter;
  actions: Action[];
};

export type Config = {
  schema: number;
  name: string;
  notes?: string;
  /** NFQWS2_COMPAT_VER the actions were written against. */
  compat: number;
  strategies: Strategy[];
};

export type EngineStatus = {
  running: boolean;
  active_config?: string;
  ephemeral: boolean;
  pid?: number;
  started_at?: number;
  engine_version?: string;
  last_error?: string;
  /** Seconds until an unconfirmed trial run is undone; absent once kept. */
  revert_in_seconds?: number;
};

export type Tool = { path: string; version?: string };

export type LuaRuntime = Tool & {
  flavour: "luajit" | "puc";
  major: number;
  minor: number;
  /** False when the engine would start and then fail to load its Lua attacks. */
  supported: boolean;
};

export type PackageManager =
  "pacman" | "apt" | "dnf" | "zypper" | "apk" | "xbps" | "portage";

export type Distro = {
  id: string;
  pretty_name?: string;
  package_manager?: PackageManager;
};

/** Absent fields mean "not on this machine". */
export type Environment = {
  platform?: "linux" | "windows" | "bsd";
  engine?: Tool;
  lua?: LuaRuntime;
  nftables?: Tool;
  distro?: Distro;
  existing_install?: string;
};

/** Local probes only; no daemon needed, which is the point during onboarding. */
export const detectEnvironment = () =>
  invoke<Environment>("detect_environment");

export type Verdict =
  | "fine"
  | "dns_failed"
  | "dns_poisoned"
  | "tcp_blocked"
  | "tls_reset"
  | "tls_silent"
  | "bad_host";

export type SiteResult = {
  host: string;
  verdict: Verdict;
  detail?: string;
  elapsed_ms: number;
};

export type Report = {
  /** False when even the control host failed: the network is down, so no other verdict means anything. */
  network_ok: boolean;
  control: SiteResult;
  sites: SiteResult[];
};

/** Seconds, not milliseconds — an unreachable host costs a full timeout. Omit `hosts` for the
 *  built-in list. Nothing is uploaded and the list never leaves the machine. */
export const checkReachability = (hosts?: string[], timeoutMs?: number) =>
  invoke<Report>("check_reachability", { hosts, timeoutMs });

/** The only port the reachability check speaks on. */
export const PROBED_TCP_PORT = "443";
/** Per-candidate timeout during the walk; the baseline check uses the daemon's longer default. */
export const TRIAL_TIMEOUT_MS = 2000;

/** Ordered most to least likely to be enough. The walk is driven from here: start each one as a
 *  trial, re-check, stop it if it did not help. */
/** A strategy to try, and how much it disturbs traffic. Cost only orders strategies that work. */
export type Candidate = { config: Config; cost: number };

export const autoconfigCandidates = () =>
  invoke<Candidate[]>("autoconfig_candidates");

export type Warning = {
  /** Null for warnings about the config as a whole. */
  strategy: string | null;
  message: string;
};

export type DaemonInfo = {
  reachable: boolean;
  version?: string;
  problem?: string;
};

/** A working config to start from — comes from Rust so it stays in step with the renderer. */
export const starterConfig = (name: string) =>
  invoke<Config>("starter_config", { name });

/** The exact nfqws2 parameter file the daemon would hand the engine. */
export const previewConfig = (config: Config) =>
  invoke<string>("preview_config", { config });

/** Lua function names this build knows, for a strategy editor. */
export const knownFunctions = () => invoke<string[]>("known_functions");

export type ServiceStatus = {
  installed: boolean;
  enabled: boolean;
  active: boolean;
  unsupported?: string;
};

export const serviceStatus = () => invoke<ServiceStatus>("service_status");
/** Both prompt via polkit. */
export const serviceSetEnabled = (enabled: boolean) =>
  invoke<void>("service_set_enabled", { enabled });
export const serviceSetActive = (active: boolean) =>
  invoke<void>("service_set_active", { active });

export type LogFile = { name: string; size: number; modified?: number };

export const listLogs = () => invoke<LogFile[]>("list_logs");
export const readLog = (name: string) => invoke<string>("read_log", { name });

/** Writes a diagnostics file locally and returns its path. Nothing is uploaded. */
export const exportDiagnostics = (logsWanted: string[]) =>
  invoke<string>("export_diagnostics", { logsWanted });

/** Resolves even when the daemon is absent — check `reachable`. */
export const daemonInfo = () => invoke<DaemonInfo>("daemon_info");

export const engineStatus = () => invoke<EngineStatus>("engine_status");
export const engineStart = (config: Config, ephemeral = false) =>
  invoke<void>("engine_start", { config, ephemeral });
/** Cancels the daemon's automatic revert and persists the config. */
export const engineConfirm = () => invoke<void>("engine_confirm");
export const engineStop = () => invoke<void>("engine_stop");

/** Local and pure; safe to call on every keystroke. */
export const lintConfig = (config: Config) =>
  invoke<Warning[]>("lint_config", { config });

export const listConfigs = () => invoke<string[]>("list_configs");
export const loadConfig = (name: string) =>
  invoke<Config>("load_config", { name });
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });
export const deleteConfig = (name: string) =>
  invoke<void>("delete_config", { name });
