import type { ReactNode } from "react";
import type { Report, SiteResult, Verdict } from "../../api";
import { BackendContext, type Backend } from "../backend";

export const SCENARIOS = ["on", "off", "trial", "no-engine", "no-daemon", "network-down", "no-configs"] as const;
export type Scenario = (typeof SCENARIOS)[number];

const site = (host: string, verdict: Verdict, ms: number): SiteResult => ({
  host,
  verdict,
  detail: verdict === "fine" ? "TLS handshake completed" : verdict === "tls_reset" ? "connection reset after the hello" : "no reply after the hello",
  elapsed_ms: ms,
});

const control = site("example.com", "fine", 71);
const open: Report = {
  network_ok: true,
  control,
  sites: [site("discord.com", "fine", 93), site("youtube.com", "fine", 93), site("x.com", "fine", 155), site("signal.org", "fine", 220), site("soundcloud.com", "fine", 132), site("medium.com", "fine", 220)],
};
const blocked: Report = {
  ...open,
  sites: [site("discord.com", "tls_reset", 2000), site("youtube.com", "tls_silent", 2000), site("x.com", "dns_poisoned", 40), site("signal.org", "fine", 220), site("soundcloud.com", "tls_reset", 2000), site("medium.com", "fine", 220)],
};
const down: Report = {
  network_ok: false,
  control: site("example.com", "dns_failed", 2000),
  sites: open.sites.map((s) => site(s.host, "dns_failed", 2000)),
};

const noop = async () => {};

function build(scenario: Scenario): Backend {
  const running = scenario === "on" || scenario === "trial";
  const base: Backend = {
    daemon: { reachable: true, version: "0.1.0" },
    status: running
      ? {
          running: true,
          active_config: "fake-badsum",
          ephemeral: scenario === "trial",
          pid: 24817,
          started_at: Math.floor(Date.now() / 1000) - 5400,
          engine_version: "nfqws2 v0.9.2",
          revert_in_seconds: scenario === "trial" ? 62 : undefined,
        }
      : { running: false, ephemeral: false },
    service: { installed: true, enabled: true, active: true },
    env: {
      platform: "linux",
      distro: { id: "arch", pretty_name: "CachyOS", package_manager: "pacman" },
      engine: { path: "/opt/zapret2/nfqws2", version: "v0.9.2" },
      lua: { path: "/usr/bin/luajit", version: "LuaJIT 2.1.0", flavour: "luajit", major: 2, minor: 1, supported: true },
      nftables: { path: "/usr/bin/nft", version: "nftables v1.1.1" },
    },
    configs: scenario === "no-configs" ? [] : ["fake-badsum", "multisplit-pos1"],
    ready: true,
    report: scenario === "on" || scenario === "trial" ? open : scenario === "network-down" ? down : blocked,
    checking: false,
    check: noop,
    refresh: noop,
    refreshEnv: noop,
    refreshConfigs: noop,
    act: async () => true,
  };
  if (scenario === "no-engine") base.env = { ...base.env, engine: undefined };
  if (scenario === "no-daemon") {
    base.daemon = { reachable: false, problem: "connect /run/blkbstr/blkbstr.sock: no such file or directory" };
    base.status = undefined;
  }
  return base;
}

/** Static backend state for rendering real screens in a chosen situation. Actions do nothing. */
export function FakeBackend({ scenario, children }: { scenario: Scenario; children: ReactNode }) {
  return <BackendContext.Provider value={build(scenario)}>{children}</BackendContext.Provider>;
}
