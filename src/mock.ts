/** Dev-only stand-in for the Tauri backend. Pick a scenario with ?mock=fresh|ready|nodaemon|noengine|legacy|biglog. */
import type { Config, EngineStatus, Report, SiteResult, Verdict } from "./api";

const scenario = new URLSearchParams(location.search).get("mock") ?? "fresh";
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const cfg = (name: string, notes: string, fn: string): Config => ({
  schema: 1,
  name,
  notes,
  compat: 1,
  strategies: [{ name: "tls", enabled: true, filter: { tcp: "443", l7: ["tls"] }, actions: [{ function: fn, args: { pos: "1", ttl: "4" } }] }],
});

const CANDIDATES: { config: Config; cost: number }[] = [
  { config: cfg("multisplit-pos1", "Split the TLS hello after the first byte", "multisplit"), cost: 1 },
  { config: cfg("multisplit-sniext", "Split at the SNI extension", "multisplit"), cost: 2 },
  { config: cfg("fakedsplit-md5sig", "Faked split with an MD5 option", "fakedsplit"), cost: 3 },
  { config: cfg("fake-badsum", "Fake hello with a bad checksum", "fake"), cost: 4 },
  { config: cfg("fake-ttl4", "Fake hello that dies after four hops", "fake"), cost: 5 },
  { config: cfg("fake-autottl", "Fake hello with an automatic TTL", "fake"), cost: 6 },
  { config: cfg("syndata", "Data in the SYN packet", "syndata"), cost: 7 },
  { config: cfg("oob-urgent", "Out-of-band byte after the hello", "oob"), cost: 8 },
];

const BLOCKED: Record<string, Verdict> = {
  "discord.com": "tls_reset",
  "youtube.com": "tls_silent",
  "x.com": "dns_poisoned",
  "soundcloud.com": "tls_reset",
};
const HOSTS = ["discord.com", "youtube.com", "x.com", "signal.org", "soundcloud.com", "medium.com"];

const saved = new Map<string, Config>((scenario === "ready" || scenario === "biglog") ? [["fake-badsum", { ...CANDIDATES[3].config, name: "fake-badsum" }], ["multisplit-pos1", CANDIDATES[0].config]] : []);
let running: { config: Config; ephemeral: boolean; startedAt: number; revertAt?: number } | undefined =
  (scenario === "ready" || scenario === "biglog") ? { config: saved.get("fake-badsum")!, ephemeral: false, startedAt: Math.floor(Date.now() / 1000) - 5400 } : undefined;

let engineInstalled = scenario !== "noengine";

const fixesFor = (name?: string) => (name?.startsWith("fake") ? 1 : name?.startsWith("multisplit") ? 0.5 : 0);

function site(host: string): SiteResult {
  const fix = fixesFor(running?.config.name);
  const verdict = BLOCKED[host];
  const open = !verdict || fix === 1 || (fix === 0.5 && host.length % 2 === 0);
  const v: Verdict = open ? "fine" : verdict;
  return { host, verdict: v, detail: v === "fine" ? "TLS handshake completed" : v === "dns_poisoned" ? "answer points at a block page" : v === "tls_reset" ? "connection reset after the hello" : "no reply after the hello", elapsed_ms: open ? 90 + (host.length * 13) % 140 : 2000 };
}

async function reachability(hosts?: string[]): Promise<Report> {
  await sleep(500);
  return { network_ok: true, control: { host: "example.com", verdict: "fine", detail: "TLS handshake completed", elapsed_ms: 71 }, sites: (hosts ?? HOSTS).map(site) };
}

function status(): EngineStatus {
  if (!running) return { running: false, ephemeral: false };
  const revert = running.revertAt ? Math.max(0, Math.round((running.revertAt - Date.now()) / 1000)) : undefined;
  if (running.revertAt && revert === 0) running = undefined;
  if (!running) return { running: false, ephemeral: false };
  return { running: true, active_config: running.config.name, ephemeral: running.ephemeral, pid: 24817, started_at: running.startedAt, engine_version: "nfqws2 v0.9.2", revert_in_seconds: revert };
}

const LOG = `2026-10-25 09:12:03 blkbstrd started, version 0.1.0
2026-10-25 09:12:03 removing leftover rules from a previous run
2026-10-25 09:12:04 starting nfqws2 with 1 profile
2026-10-25 09:12:04 nft: table inet blkbstr installed, queue 200
2026-10-25 09:12:04 engine ready, pid 24817
2026-10-25 09:40:18 warn: profile tls: no packets in 20 minutes
2026-10-25 10:02:51 error: nfqws2 exited with status 1, restarting in 2s
2026-10-25 10:02:53 starting nfqws2 with 1 profile
2026-10-25 10:02:54 engine ready, pid 25102`;

export async function mock<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await sleep(40);
  const out = (v: unknown) => v as T;
  switch (cmd) {
    case "daemon_info":
      return out(scenario === "nodaemon" ? { reachable: false, problem: "connect /run/blkbstr/blkbstr.sock: no such file or directory" } : { reachable: true, version: "0.1.0" });
    case "service_status":
      return out({ installed: scenario !== "nodaemon", enabled: scenario === "ready", active: scenario !== "nodaemon" });
    case "service_install":
    case "service_set_enabled":
    case "service_set_active":
    case "engine_confirm":
      if (cmd === "engine_confirm" && running) {
        running.revertAt = undefined;
        running.ephemeral = false;
      }
      return out(undefined);
    case "detect_environment":
      if (new URLSearchParams(location.search).get("platform") === "windows") {
        return out({
          platform: "windows",
          engine: engineInstalled ? { path: "C:\\Program Files\\zapret2\\winws2.exe", version: "v0.9.2" } : undefined,
        });
      }
      return out({
        platform: "linux",
        distro: { id: "arch", pretty_name: "CachyOS", package_manager: "pacman" },
        engine: engineInstalled ? { path: "/opt/zapret2/nfqws2", version: "v0.9.2" } : undefined,
        existing_install: scenario === "legacy" || scenario === "noengine" ? "/opt/zapret2" : undefined,
        lua: { path: "/usr/bin/luajit", version: "LuaJIT 2.1.0", flavour: "luajit", major: 2, minor: 1, supported: true },
        nftables: { path: "/usr/bin/nft", version: "nftables v1.1.1" },
      });
    case "install_engine":
      await sleep(2200);
      if (scenario === "noengine" && new URLSearchParams(location.search).has("fail")) throw "the authorisation prompt was dismissed";
      engineInstalled = true;
      return out(undefined);
    case "import_legacy": {
      await sleep(400);
      const config = cfg("Legacy", "Imported from /opt/zapret2/config", "multisplit");
      saved.set("Legacy", config);
      return out({
        name: "Legacy",
        backup: "/home/user/.config/blkbstr/backups/zapret2-config-1790000000",
        skipped: scenario === "legacy" ? ["--ctrack-timeouts=1", "<HOSTLIST> (list selection is not imported)"] : [],
      });
    }
    case "check_reachability":
      return out(await reachability(args.hosts as string[] | undefined));
    case "autoconfig_candidates":
      return out(CANDIDATES);
    case "engine_start": {
      const c = args.config as Config;
      const ephemeral = Boolean(args.ephemeral);
      running = { config: c, ephemeral, startedAt: Math.floor(Date.now() / 1000), revertAt: ephemeral ? Date.now() + 90_000 : undefined };
      return out(undefined);
    }
    case "engine_stop":
      running = undefined;
      return out(undefined);
    case "engine_status":
      return out(status());
    case "starter_config":
      return out(CANDIDATES[3].config);
    case "preview_config":
      return out(`--filter-tcp=443\n--filter-l7=tls\n--lua-desync=${(args.config as Config).strategies[0]?.actions[0]?.function}:pos=1:ttl=4\n`);
    case "known_functions":
      return out(["fake", "multisplit", "fakedsplit", "syndata", "oob", "wssize"]);
    case "lint_config":
      return out([]);
    case "list_configs":
      return out([...saved.keys()]);
    case "load_config":
      return out(saved.get(args.name as string));
    case "save_config":
      saved.set((args.config as Config).name, args.config as Config);
      return out(undefined);
    case "delete_config":
      saved.delete(args.name as string);
      return out(undefined);
    case "list_logs":
      return out([{ name: "blkbstrd.log", size: 2048, modified: Date.now() / 1000 }, { name: "nfqws2.log", size: 1024, modified: Date.now() / 1000 }]);
    case "read_log":
      // ?mock=biglog: a log the size of the real tail window, to look at scrolling and stutter.
      return out(scenario === "biglog" ? Array.from({ length: 6000 }, (_, i) => `2026-10-25 10:${String(i % 60).padStart(2, "0")}:00 ${i % 97 === 0 ? "error: nfqws2 exited, restarting" : i % 31 === 0 ? "warn: no packets for a while" : "profile tls: handled a packet from 10.0.0.2 to 203.0.113.9 port 443"} #${i}`).join("\n") : LOG);
    case "export_diagnostics":
      return out("/tmp/blkbstr-diagnostics.txt");
    default:
      throw new Error(`mock: no handler for ${cmd}`);
  }
}
