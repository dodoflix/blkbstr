/** Typed mirror of the `#[tauri::command]`s in `src-tauri/src/lib.rs`. */
import { invoke } from "./ipc";

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
  /** Work around only sites the engine has seen fail; on by default. */
  auto_hostlist: boolean;
  /** Work around only the sites on the user's own list; off by default. Combines with `auto_hostlist`. */
  manual_hostlist: boolean;
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

/** `auto` is what the engine learned is blocked; `manual` is the user's own list. */
export type HostList = "auto" | "manual";

export type HostsStatus = {
  list: HostList;
  /** True when the running config uses the list. */
  enabled: boolean;
  hosts: string[];
  /** False where the daemon cannot probe without disturbing traffic (neither Linux nor Windows). */
  can_recheck: boolean;
  /** Only the learned list is re-checked. */
  checking: boolean;
  /** Unix seconds the last re-check finished. */
  last_check?: number;
  /** Hosts the last re-check found loading without a strategy, and dropped. */
  last_dropped: string[];
  /** Unix seconds the next scheduled re-check runs. */
  next_check?: number;
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
  /** The zapret2 release this build pins, set when Blockbuster installed a different one. */
  engine_update?: string;
};

export const detectEnvironment = () =>
  invoke<Environment>("detect_environment");

/** Downloads the pinned zapret2 release and installs it after a polkit or UAC prompt. Resolves once it is in place. */
export const installEngine = () => invoke<void>("install_engine");

export type LegacyImport = {
  /** The name the config was saved under. */
  name: string;
  /** Where the original zapret2 config was copied. */
  backup: string;
  /** Options that did not map onto a config, verbatim. */
  skipped: string[];
};

export type AppUpdate = {
  version: string;
  notes?: string;
  /** False for deb and rpm installs, which update through their package manager. */
  installable: boolean;
};

/** Null when this is the newest release. Talks to GitHub Releases only. */
export const checkAppUpdate = () => invoke<AppUpdate | null>("check_app_update");
/** Downloads, verifies the signature, installs and restarts the app. */
export const installAppUpdate = () => invoke<void>("install_app_update");

export const importLegacy = () => invoke<LegacyImport>("import_legacy");

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
  /** False when even the control host failed: the network is down, so no verdict means anything. */
  network_ok: boolean;
  control: SiteResult;
  sites: SiteResult[];
};

/** Timeout in ms; an unreachable host costs a full one. Omit `hosts` for the built-in list. Nothing
 *  is uploaded. */
export const checkReachability = (hosts?: string[], timeoutMs?: number) =>
  invoke<Report>("check_reachability", { hosts, timeoutMs });

/** A group of built-in sites, so a whole kind of site can be left out of the check. */
export type SiteCategory = { id: string; label: string; hosts: string[] };

export const siteCategories = () => invoke<SiteCategory[]>("site_categories");

/** The only port the reachability check speaks on. */
export const PROBED_TCP_PORT = "443";
/** Per-candidate timeout during the walk; the baseline check uses the daemon's longer default. */
export const TRIAL_TIMEOUT_MS = 2000;

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
  /** The helper runs a different version than this app. */
  outdated?: boolean;
  problem?: string;
};

/** A working config to start from; comes from Rust so it stays in step with the renderer. */
export const starterConfig = (name: string) =>
  invoke<Config>("starter_config", { name });

/** The exact nfqws2 parameter file the daemon would hand the engine. */
export const previewConfig = (config: Config) =>
  invoke<string>("preview_config", { config });

export const knownFunctions = () => invoke<string[]>("known_functions");

export type ServiceStatus = {
  installed: boolean;
  enabled: boolean;
  active: boolean;
  unsupported?: string;
};

export const serviceStatus = () => invoke<ServiceStatus>("service_status");
/** Registers the background helper on Windows, behind one UAC prompt. Linux installs it with a script. */
export const serviceInstall = () => invoke<void>("service_install");
/** Both prompt: polkit on Linux, UAC on Windows (starting and stopping on Windows do not). */
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

/** Resolves even when the daemon is absent - check `reachable`. */
export const daemonInfo = () => invoke<DaemonInfo>("daemon_info");

export const engineStatus = () => invoke<EngineStatus>("engine_status");
export const engineStart = (config: Config, ephemeral = false) =>
  invoke<void>("engine_start", { config, ephemeral });
/** Cancels the daemon's automatic revert and persists the config. */
export const engineConfirm = () => invoke<void>("engine_confirm");
export const engineStop = () => invoke<void>("engine_stop");

export const hosts = (list: HostList) => invoke<HostsStatus>("hosts", { list });
/** Anything that is not a plain hostname is ignored by the daemon. */
export const hostsAdd = (list: HostList, hosts: string[]) => invoke<void>("hosts_add", { list, hosts });
/** The engine learns a removed host again if it still fails. */
export const hostsRemove = (list: HostList, hosts: string[]) => invoke<void>("hosts_remove", { list, hosts });
/** Re-checks the learned list now. Returns at once; progress shows in `hosts`. */
export const hostsCheck = () => invoke<void>("hosts_check");

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
