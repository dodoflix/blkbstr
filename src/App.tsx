import { useCallback, useEffect, useRef, useState } from "react";
import {
  Badge,
  Button,
  Callout,
  Card,
  Checkbox,
  Code,
  Container,
  DataList,
  Flex,
  Heading,
  RadioGroup,
  ScrollArea,
  Select,
  Tabs,
  Text,
  TextField,
} from "@radix-ui/themes";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import * as api from "./api";

/** Coarse on purpose: "3h" is what a user wants from an uptime, not "3h 07m 41s". */
function uptime(since: number): string {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - since);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

export default function App() {
  return (
    <Container size="3" p="5">
      <Flex direction="column" gap="4">
        <Flex align="center" gap="3">
          <img src="/icon.svg" alt="" width="40" height="40" />
          <Flex align="baseline" gap="3">
            <Heading size="7">Blockbuster</Heading>
            <Text size="2" color="gray">
              zapret2, with a front door
            </Text>
          </Flex>
        </Flex>

        <Tabs.Root defaultValue="status">
          <Tabs.List>
            <Tabs.Trigger value="status">Status</Tabs.Trigger>
            <Tabs.Trigger value="setup">Setup</Tabs.Trigger>
            <Tabs.Trigger value="configs">Configs</Tabs.Trigger>
            <Tabs.Trigger value="logs">Logs</Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="status" mt="4">
            <StatusPanel />
          </Tabs.Content>
          <Tabs.Content value="setup" mt="4">
            <Flex direction="column" gap="4">
              <SetupPanel />
              <AutoConfig />
            </Flex>
          </Tabs.Content>
          <Tabs.Content value="configs" mt="4">
            <ConfigsPanel />
          </Tabs.Content>
          <Tabs.Content value="logs" mt="4">
            <LogsPanel />
          </Tabs.Content>
        </Tabs.Root>
      </Flex>
    </Container>
  );
}

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** The rules are already installed when engineStart returns; this is only for the engine to be
 *  reading the queue. Measured at well under 250ms, so this is mostly margin. */
const SETTLE_MS = 600;

/** A candidate that filters ports the check never speaks on cannot change what the check measures,
 *  so trying it costs a full timeout to learn nothing. */
function measurable(config: api.Config): boolean {
  return config.strategies.some(
    (s) =>
      s.enabled &&
      (s.filter.tcp ?? "").split(",").includes(api.PROBED_TCP_PORT),
  );
}

type Step = {
  name: string;
  notes?: string;
  outcome: string;
  /** Undefined while the candidate is still being tried. */
  worked?: boolean;
  unblocked: number;
  cost: number;
  config: api.Config;
};

/** Best first: more sites through wins, then the strategy that disturbs traffic least, then the
 *  earlier one, which is the hand-ranked order.
 *
 *  Includes strategies that only got some of the sites through. When nothing gets everything, the
 *  one that gets most of it is still the best answer available, and hiding it helps nobody. */
function rank(steps: Step[]): Step[] {
  return steps
    .filter((step) => step.unblocked > 0)
    .sort((a, b) => b.unblocked - a.unblocked || a.cost - b.cost);
}

/** Tries every candidate and reports all of them that worked.
 *
 *  It does not stop at the first success: the first strategy that gets through is not necessarily
 *  the one to live with, and which trade-off is acceptable is the user's call, not a heuristic's.
 *
 *  The loop lives here rather than in the daemon because every step is already an ordinary
 *  start / check / stop: driving it from the UI costs no privileged surface, makes progress and
 *  cancellation free, and if this window dies mid-walk the trial run's own deadline puts the
 *  machine back. */
function AutoConfig() {
  const [steps, setSteps] = useState<Step[]>([]);
  const [running, setRunning] = useState(false);
  const [winner, setWinner] = useState<api.Config>();
  const [saveAs, setSaveAs] = useState("auto");
  const [note, setNote] = useState<string>();
  const [error, setError] = useState<string>();
  const [blockedCount, setBlockedCount] = useState(0);
  const [progress, setProgress] = useState<[number, number]>([0, 0]);
  // A ref, not state: the walk reads it inside a loop that never re-renders.
  const cancelled = useRef(false);

  const finish = (
    name: string,
    outcome: string,
    worked = false,
    unblocked = 0,
  ) =>
    setSteps((prev) =>
      prev.map((step) =>
        step.name === name ? { ...step, outcome, worked, unblocked } : step,
      ),
    );

  const run = async () => {
    cancelled.current = false;
    setRunning(true);
    setProgress([0, 0]);
    setSteps([]);
    setWinner(undefined);
    setNote(undefined);
    setError(undefined);
    try {
      const baseline = await api.checkReachability();
      if (!baseline.network_ok) {
        setError(
          `Even ${baseline.control.host} did not answer, so this is the network rather than a censor. Nothing can be measured until that is fixed.`,
        );
        return;
      }
      const blocked = baseline.sites
        .filter((site) => site.verdict !== "fine")
        .map((site) => site.host);
      setBlockedCount(blocked.length);
      if (blocked.length === 0) {
        setNote(
          "Every site on the list already loads. There is nothing to work around.",
        );
        return;
      }

      const tried: Step[] = [];
      const all = await api.autoconfigCandidates();
      const usable = all.filter(({ config }) => measurable(config));
      const skipped = all.length - usable.length;
      setProgress([0, usable.length]);
      for (const [i, { config, cost }] of usable.entries()) {
        if (cancelled.current) break;
        setProgress([i + 1, usable.length]);
        setSteps((prev) => [
          ...prev,
          {
            name: config.name,
            notes: config.notes,
            outcome: "trying…",
            unblocked: 0,
            cost,
            config,
          },
        ]);
        let step: Step = {
          name: config.name,
          notes: config.notes,
          outcome: "",
          unblocked: 0,
          cost,
          config,
        };
        try {
          await api.engineStart(config, true);
        } catch (e) {
          // The engine refuses a config it cannot load before any rule is installed. That is a
          // candidate to skip, not a reason to stop.
          finish(config.name, `the engine refused it: ${e}`);
          continue;
        }
        await sleep(SETTLE_MS);
        const report = await api.checkReachability(
          blocked,
          api.TRIAL_TIMEOUT_MS,
        );
        // Always stop: every candidate is measured against the same untouched machine, and the
        // one the user settles on is started again by name when they keep it.
        await api.engineStop();

        const unblocked = report.sites.filter(
          (site) => site.verdict === "fine",
        ).length;
        const worked = unblocked === blocked.length;
        step = { ...step, worked, unblocked };
        tried.push(step);
        finish(
          config.name,
          worked
            ? `all ${blocked.length} load`
            : `${blocked.length - unblocked} of ${blocked.length} still blocked`,
          worked,
          unblocked,
        );
      }

      const unmeasured =
        skipped > 0
          ? ` ${skipped} that only touch other ports were skipped: the check speaks TLS on ${api.PROBED_TCP_PORT} and nothing else, so it could not tell whether they helped.`
          : "";
      const stopped = cancelled.current
        ? ` Stopped after ${tried.length} of ${usable.length}.`
        : "";
      const helped = rank(tried);
      if (helped.length === 0) {
        setError(
          `No strategy got anything through. The engine is stopped and the machine is back as it was.${stopped}${unmeasured}`,
        );
        return;
      }
      setWinner(helped[0].config);
      setSaveAs(helped[0].config.name);
      const best = helped[0].unblocked;
      setNote(
        best === blocked.length
          ? `${helped.filter((s) => s.worked).length} of ${tried.length} strategies got everything through. The gentlest is picked for you; any of them can be used instead.${stopped}`
          : `Nothing got all ${blocked.length} through. The best got ${best}. Pick one or try again later — a censor is not the same from one hour to the next.${stopped}${unmeasured}`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  /** Restarts under the chosen name before confirming, so the saved file, the running config and
   *  the one that comes back at boot all say the same thing. */
  const keep = async () => {
    if (!winner) return;
    const named = { ...winner, name: saveAs };
    try {
      await api.engineStart(named, true);
      await api.engineConfirm();
      await api.saveConfig(named);
      setNote(`Saved as ${saveAs} and kept. It will come back after a reboot.`);
      setWinner(undefined);
    } catch (e) {
      setError(String(e));
    }
  };

  const working = rank(steps);

  return (
    <Card>
      <Flex direction="column" gap="3">
        <Flex align="center" gap="3" wrap="wrap">
          <Button onClick={() => void run()} loading={running}>
            Find a working strategy
          </Button>
          {running && (
            <>
              <Button
                variant="soft"
                color="gray"
                onClick={() => (cancelled.current = true)}
              >
                Stop
              </Button>
              <Text size="2">
                {progress[0]} of {progress[1]}
              </Text>
            </>
          )}
          <Text size="1" color="gray">
            Tries every strategy for a couple of seconds each, then lists the
            ones that got the blocked sites through, gentlest first. The likely
            ones come early, so stopping partway still gives an answer. Nothing
            is kept until you say so.
          </Text>
        </Flex>

        {note && (
          <Callout.Root color="jade">
            <Callout.Text>{note}</Callout.Text>
          </Callout.Root>
        )}
        {error && (
          <Callout.Root color="amber">
            <Callout.Text>{error}</Callout.Text>
          </Callout.Root>
        )}

        {working.length > 0 && (
          <Flex direction="column" gap="2">
            <Heading size="2">Strategies that helped</Heading>
            <RadioGroup.Root
              value={winner?.name ?? ""}
              onValueChange={(name) => {
                const picked = working.find((step) => step.name === name);
                if (picked) {
                  setWinner(picked.config);
                  setSaveAs(picked.name);
                }
              }}
            >
              <Flex direction="column" gap="2">
                {working.map((step, i) => (
                  <RadioGroup.Item key={step.name} value={step.name}>
                    <Flex align="center" gap="2" wrap="wrap">
                      <Text size="2">{step.name}</Text>
                      {i === 0 && <Badge color="jade">gentlest</Badge>}
                      {!step.worked && (
                        <Badge color="amber">
                          {step.unblocked} of {blockedCount}
                        </Badge>
                      )}
                      <Text size="1" color="gray">
                        disturbance {step.cost} · {step.notes}
                      </Text>
                    </Flex>
                  </RadioGroup.Item>
                ))}
              </Flex>
            </RadioGroup.Root>
          </Flex>
        )}

        {steps.length > 0 && (
          <DataList.Root>
            {steps.map((step) => (
              <DataList.Item key={step.name}>
                <DataList.Label>{step.name}</DataList.Label>
                <DataList.Value>
                  <Flex direction="column">
                    <Flex align="center" gap="2" wrap="wrap">
                      <Badge color={step.worked ? "jade" : "gray"}>
                        {step.outcome}
                      </Badge>
                    </Flex>
                    {step.notes && (
                      <Text size="1" color="gray">
                        {step.notes}
                      </Text>
                    )}
                  </Flex>
                </DataList.Value>
              </DataList.Item>
            ))}
          </DataList.Root>
        )}

        {winner && (
          <Flex align="center" gap="2" wrap="wrap">
            <Text size="2">Save it as</Text>
            <TextField.Root
              value={saveAs}
              onChange={(e) => setSaveAs(e.currentTarget.value)}
              style={{ width: "12rem" }}
            />
            <Button onClick={() => void keep()} disabled={!saveAs.trim()}>
              Save and keep
            </Button>
            <Text size="1" color="gray">
              Until you do, it reverts on its own.
            </Text>
          </Flex>
        )}
      </Flex>
    </Card>
  );
}

/** What each verdict means in one phrase, and how alarmed to look about it. */
const VERDICTS: Record<
  api.Verdict,
  { label: string; color: "jade" | "amber" | "red" }
> = {
  fine: { label: "reachable", color: "jade" },
  dns_failed: { label: "DNS failed", color: "amber" },
  dns_poisoned: { label: "DNS poisoned", color: "red" },
  tcp_blocked: { label: "TCP blocked", color: "red" },
  tls_reset: { label: "reset after SNI", color: "red" },
  tls_silent: { label: "no answer", color: "red" },
  bad_host: { label: "bad hostname", color: "amber" },
};

function SiteRow({ site }: { site: api.SiteResult }) {
  const verdict = VERDICTS[site.verdict];
  return (
    <DataList.Item>
      <DataList.Label>{site.host}</DataList.Label>
      <DataList.Value>
        <Flex align="center" gap="2" wrap="wrap">
          <Badge color={verdict.color}>{verdict.label}</Badge>
          <Text size="1" color="gray">
            {site.detail} · {site.elapsed_ms}ms
          </Text>
        </Flex>
      </DataList.Value>
    </DataList.Item>
  );
}

/** Green when it is there, amber when it is not, red when it is there and unusable. */
function Detected({ tool, missing }: { tool?: api.Tool; missing: string }) {
  if (!tool) return <Badge color="amber">{missing}</Badge>;
  return (
    <Flex align="center" gap="2" wrap="wrap">
      <Badge color="jade">found</Badge>
      <Text size="1" color="gray">
        {tool.version ?? tool.path}
      </Text>
    </Flex>
  );
}

/** What is on this machine. Runs entirely locally, so it works before the service is installed —
 *  which is exactly when someone needs to know what is missing. */
function SetupPanel() {
  const [env, setEnv] = useState<api.Environment>();
  const [report, setReport] = useState<api.Report>();
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string>();

  const runCheck = async () => {
    setChecking(true);
    try {
      setReport(await api.checkReachability());
      setError(undefined);
    } catch (e) {
      setError(String(e));
    }
    setChecking(false);
  };

  const refresh = useCallback(async () => {
    try {
      setEnv(await api.detectEnvironment());
      setError(undefined);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  if (!env) {
    return (
      <Text size="2" color="gray">
        {error ?? "Looking at this machine\u2026"}
      </Text>
    );
  }

  const lua = env.lua;
  return (
    <Flex direction="column" gap="3">
      {error && (
        <Callout.Root color="red">
          <Callout.Text>{error}</Callout.Text>
        </Callout.Root>
      )}

      {!env.engine && (
        <Callout.Root color="amber">
          <Callout.Text>
            zapret2 is not installed. Blockbuster drives the upstream engine and
            does not ship one; put <Code>nfqws2</Code> on <Code>PATH</Code> or
            in <Code>/opt/zapret2</Code>.
          </Callout.Text>
        </Callout.Root>
      )}

      {lua && !lua.supported && (
        <Callout.Root color="red">
          <Callout.Text>
            {lua.version} is too old. zapret2's strategies are Lua, so the
            engine will start and then fail to load them — LuaJIT 2.1+ or Lua
            5.3+ is needed.
          </Callout.Text>
        </Callout.Root>
      )}

      <Card>
        <DataList.Root>
          <DataList.Item>
            <DataList.Label>System</DataList.Label>
            <DataList.Value>
              <Flex align="center" gap="2" wrap="wrap">
                <Badge color={env.platform ? "jade" : "red"}>
                  {env.platform ?? "unsupported platform"}
                </Badge>
                {env.distro && (
                  <Text size="1" color="gray">
                    {env.distro.pretty_name ?? env.distro.id}
                    {env.distro.package_manager &&
                      ` \u00b7 ${env.distro.package_manager}`}
                  </Text>
                )}
              </Flex>
            </DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Engine</DataList.Label>
            <DataList.Value>
              <Detected tool={env.engine} missing="not installed" />
            </DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>Lua</DataList.Label>
            <DataList.Value>
              {lua ? (
                <Flex align="center" gap="2" wrap="wrap">
                  <Badge color={lua.supported ? "jade" : "red"}>
                    {lua.supported ? "found" : "too old"}
                  </Badge>
                  <Text size="1" color="gray">
                    {lua.version ?? lua.path}
                  </Text>
                </Flex>
              ) : (
                <Badge color="amber">not installed</Badge>
              )}
            </DataList.Value>
          </DataList.Item>

          <DataList.Item>
            <DataList.Label>nftables</DataList.Label>
            <DataList.Value>
              <Detected tool={env.nftables} missing="not installed" />
            </DataList.Value>
          </DataList.Item>

          {env.existing_install && (
            <DataList.Item>
              <DataList.Label>Existing install</DataList.Label>
              <DataList.Value>
                <Text size="1" color="gray">
                  {env.existing_install}
                </Text>
              </DataList.Value>
            </DataList.Item>
          )}
        </DataList.Root>
      </Card>

      <Flex align="center" gap="3">
        <Button variant="soft" onClick={() => void refresh()}>
          Re-check
        </Button>
        <Button onClick={() => void runCheck()} loading={checking}>
          Check reachability
        </Button>
      </Flex>

      {report && (
        <Card>
          {!report.network_ok && (
            <Callout.Root color="amber" mb="3">
              <Callout.Text>
                Even <Code>{report.control.host}</Code> did not answer, so this
                is the network rather than a censor. Nothing below means
                anything until that is fixed.
              </Callout.Text>
            </Callout.Root>
          )}
          <DataList.Root>
            {report.sites.map((site) => (
              <SiteRow key={site.host} site={site} />
            ))}
            <SiteRow site={report.control} />
          </DataList.Root>
        </Card>
      )}
    </Flex>
  );
}

/** Coarse, because the status is polled every few seconds and a per-second countdown would
 *  visibly stutter. Approximate is honest here; the daemon holds the real deadline. */
function countdown(seconds: number): string {
  if (seconds >= 60) return `about ${Math.round(seconds / 60)} min`;
  return `${seconds}s`;
}

function StatusPanel() {
  const [daemon, setDaemon] = useState<api.DaemonInfo>();
  const [status, setStatus] = useState<api.EngineStatus>();
  const [svc, setSvc] = useState<api.ServiceStatus>();
  const [error, setError] = useState<string>();

  const refresh = useCallback(async () => {
    const info = await api.daemonInfo();
    setDaemon(info);
    api.serviceStatus().then(setSvc, () => setSvc(undefined));
    if (!info.reachable) {
      setStatus(undefined);
      return;
    }
    try {
      setStatus(await api.engineStatus());
      setError(undefined);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    // The daemon does not push, and uptime is a lie the moment it stops being re-read.
    const timer = setInterval(() => void refresh(), 5000);
    return () => clearInterval(timer);
  }, [refresh]);

  const act = async (fn: () => Promise<void>) => {
    try {
      await fn();
      setError(undefined);
    } catch (e) {
      setError(String(e));
    }
    await refresh();
  };

  return (
    <Flex direction="column" gap="3">
      {daemon && !daemon.reachable && (
        <Callout.Root color="amber">
          <Callout.Text>
            The privileged service is not reachable, so nothing can be started.
            Install it with <Code>packaging/linux/install.sh</Code>.
            {daemon.problem && (
              <>
                <br />
                <Text size="1" color="gray">
                  {daemon.problem}
                </Text>
              </>
            )}
          </Callout.Text>
        </Callout.Root>
      )}

      {error && (
        <Callout.Root color="red">
          <Callout.Text>{error}</Callout.Text>
        </Callout.Root>
      )}

      <Card>
        <DataList.Root>
          <DataList.Item>
            <DataList.Label>Service</DataList.Label>
            <DataList.Value>
              <Flex align="center" gap="2" wrap="wrap">
                {daemon?.reachable ? (
                  <Badge color="jade">running · v{daemon.version}</Badge>
                ) : (
                  <Badge color="gray">not reachable</Badge>
                )}
                {svc?.installed && (
                  <Badge color={svc.enabled ? "jade" : "gray"} variant="soft">
                    {svc.enabled ? "starts at boot" : "manual start"}
                  </Badge>
                )}
              </Flex>
            </DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Engine</DataList.Label>
            <DataList.Value>
              <Flex align="center" gap="2">
                {status?.running ? (
                  <Badge color="jade">active</Badge>
                ) : (
                  <Badge color="gray">stopped</Badge>
                )}
                {status?.running && status.started_at && (
                  <Text size="2" color="gray">
                    up {uptime(status.started_at)}
                  </Text>
                )}
                {status?.pid && (
                  <Text size="1" color="gray">
                    pid {status.pid}
                  </Text>
                )}
              </Flex>
            </DataList.Value>
          </DataList.Item>
          {status?.engine_version && (
            <DataList.Item>
              <DataList.Label>Version</DataList.Label>
              <DataList.Value>
                <Code size="2">{status.engine_version}</Code>
              </DataList.Value>
            </DataList.Item>
          )}
          {status?.last_error && (
            <DataList.Item>
              <DataList.Label>Last error</DataList.Label>
              <DataList.Value>
                <Text color="red" size="2">
                  {status.last_error}
                </Text>
              </DataList.Value>
            </DataList.Item>
          )}
          {status?.active_config && (
            <DataList.Item>
              <DataList.Label>Config</DataList.Label>
              <DataList.Value>
                {status.active_config}
                {status.ephemeral && (
                  <Badge ml="2" color="orange">
                    ephemeral
                  </Badge>
                )}
              </DataList.Value>
            </DataList.Item>
          )}
        </DataList.Root>
      </Card>

      {status?.revert_in_seconds !== undefined && (
        <Callout.Root color="orange">
          <Callout.Text>
            <Flex align="center" gap="3" wrap="wrap">
              <Text>
                Trying <Code>{status.active_config}</Code>. It reverts on its
                own in {countdown(status.revert_in_seconds)} unless you keep it
                — so if this broke your connection, doing nothing fixes it.
              </Text>
              <Button size="1" onClick={() => void act(api.engineConfirm)}>
                Keep it
              </Button>
              <Button
                size="1"
                variant="soft"
                color="red"
                onClick={() => void act(api.engineStop)}
              >
                Undo now
              </Button>
            </Flex>
          </Callout.Text>
        </Callout.Root>
      )}

      {svc?.installed && (
        <Flex gap="2" align="center" wrap="wrap">
          <Button
            variant="soft"
            onClick={() => void act(() => api.serviceSetActive(!svc.active))}
          >
            {svc.active ? "Stop service" : "Start service"}
          </Button>
          <Button
            variant="soft"
            onClick={() => void act(() => api.serviceSetEnabled(!svc.enabled))}
          >
            {svc.enabled ? "Don't start at boot" : "Start at boot"}
          </Button>
          <Text size="1" color="gray">
            Both ask for authentication.
          </Text>
        </Flex>
      )}

      <Flex gap="2">
        <Button onClick={() => void refresh()} variant="soft">
          Refresh
        </Button>
        <Button
          disabled={!daemon?.reachable || status?.running}
          onClick={() =>
            void act(async () =>
              api.engineStart(await api.starterConfig("default")),
            )
          }
        >
          Start
        </Button>
        <Button
          variant="soft"
          disabled={!daemon?.reachable || status?.running}
          onClick={() =>
            void act(async () =>
              api.engineStart(await api.starterConfig("trial"), true),
            )
          }
        >
          Try it
        </Button>
        <Button
          disabled={!daemon?.reachable || !status?.running}
          color="red"
          variant="soft"
          onClick={() => void act(api.engineStop)}
        >
          Stop
        </Button>
      </Flex>
    </Flex>
  );
}

function ConfigsPanel() {
  const [names, setNames] = useState<string[]>([]);
  const [active, setActive] = useState<string>();
  const [open, setOpen] = useState<string>();
  const [preview, setPreview] = useState("");
  const [busy, setBusy] = useState<string>();
  const [note, setNote] = useState<string>();
  const [confirming, setConfirming] = useState<string>();
  const [error, setError] = useState<string>();

  const refresh = useCallback(() => {
    api.listConfigs().then(setNames, (e) => setError(String(e)));
    api.engineStatus().then(
      (status) => setActive(status.running ? status.active_config : undefined),
      () => setActive(undefined),
    );
  }, []);

  useEffect(refresh, [refresh]);

  /** Shows what the engine will actually be given, which is the only way to tell two
   *  similarly-named configs apart without opening the file. */
  const show = async (name: string) => {
    if (open === name) {
      setOpen(undefined);
      return;
    }
    try {
      setPreview(await api.previewConfig(await api.loadConfig(name)));
      setOpen(name);
    } catch (e) {
      setError(String(e));
    }
  };

  const use = async (name: string) => {
    setBusy(name);
    setError(undefined);
    try {
      await api.engineStart(await api.loadConfig(name));
      setNote(`${name} is running.`);
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(undefined);
    }
  };

  const remove = async (name: string) => {
    setBusy(name);
    setError(undefined);
    try {
      await api.deleteConfig(name);
      setNote(`Deleted ${name}.`);
      setConfirming(undefined);
      if (open === name) setOpen(undefined);
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(undefined);
    }
  };

  return (
    <Card>
      <Flex direction="column" gap="3">
        {note && (
          <Callout.Root color="jade">
            <Callout.Text>{note}</Callout.Text>
          </Callout.Root>
        )}
        {error && (
          <Callout.Root color="red">
            <Callout.Text>{error}</Callout.Text>
          </Callout.Root>
        )}

        {names.length === 0 ? (
          <Text color="gray">
            No saved configs yet. Find a working strategy on the Setup tab and
            save it, or drop JSON files in the configs directory.
          </Text>
        ) : (
          <Flex direction="column" gap="2">
            {names.map((name) => (
              <Flex key={name} direction="column" gap="2">
                <Flex align="center" gap="2" wrap="wrap">
                  <Text weight="medium">{name}</Text>
                  {name === active && <Badge color="jade">running</Badge>}
                  <Flex gap="2" ml="auto">
                    <Button
                      size="1"
                      variant="soft"
                      onClick={() => void show(name)}
                    >
                      {open === name ? "Hide" : "Show"}
                    </Button>
                    <Button
                      size="1"
                      disabled={name === active}
                      loading={busy === name}
                      onClick={() => void use(name)}
                    >
                      Use
                    </Button>
                    {confirming === name ? (
                      <>
                        <Button
                          size="1"
                          color="red"
                          loading={busy === name}
                          onClick={() => void remove(name)}
                        >
                          Delete for good
                        </Button>
                        <Button
                          size="1"
                          variant="soft"
                          onClick={() => setConfirming(undefined)}
                        >
                          Cancel
                        </Button>
                      </>
                    ) : (
                      <Button
                        size="1"
                        color="red"
                        variant="soft"
                        onClick={() => setConfirming(name)}
                      >
                        Delete
                      </Button>
                    )}
                  </Flex>
                </Flex>
                {open === name && (
                  <ScrollArea style={{ maxHeight: "16rem" }}>
                    <Code
                      size="1"
                      style={{
                        whiteSpace: "pre",
                        display: "block",
                        padding: "0.5rem",
                      }}
                    >
                      {preview}
                    </Code>
                  </ScrollArea>
                )}
              </Flex>
            ))}
          </Flex>
        )}
      </Flex>
    </Card>
  );
}

function LogsPanel() {
  const [files, setFiles] = useState<api.LogFile[]>([]);
  const [selected, setSelected] = useState<string>();
  const [text, setText] = useState("");
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const [error, setError] = useState<string>();
  const [exported, setExported] = useState<string>();

  useEffect(() => {
    api.listLogs().then(
      (found) => {
        setFiles(found);
        setSelected((current) => current ?? found[0]?.name);
      },
      (e) => setError(String(e)),
    );
  }, []);

  useEffect(() => {
    if (!selected) return;
    const load = () =>
      api.readLog(selected).then(setText, (e) => setError(String(e)));
    void load();
    if (!follow) return;
    const timer = setInterval(load, 2000);
    return () => clearInterval(timer);
  }, [selected, follow]);

  // Plain substring, case-insensitive. A regex box invites a bad regex hanging the webview on a
  // 256 KB string, and "what happened around nfqws2" is what people actually type.
  const needle = filter.toLowerCase();
  const lines = text
    .split("\n")
    .filter((l) => !needle || l.toLowerCase().includes(needle));

  if (files.length === 0 && !error) {
    return (
      <Card>
        <Text color="gray">
          No logs yet. The service writes them once it has run.
        </Text>
      </Card>
    );
  }

  return (
    <Flex direction="column" gap="3">
      {error && (
        <Callout.Root color="red">
          <Callout.Text>{error}</Callout.Text>
        </Callout.Root>
      )}

      <Flex gap="2" align="center" wrap="wrap">
        <Select.Root value={selected} onValueChange={setSelected}>
          <Select.Trigger placeholder="Pick a log" />
          <Select.Content>
            {files.map((f) => (
              <Select.Item key={f.name} value={f.name}>
                {f.name}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>

        <TextField.Root
          placeholder="Filter lines…"
          value={filter}
          onChange={(e) => setFilter(e.currentTarget.value)}
          style={{ flex: 1, minWidth: 180 }}
        />

        <Text as="label" size="2">
          <Flex gap="2" align="center">
            <Checkbox
              checked={follow}
              onCheckedChange={(v) => setFollow(v === true)}
            />
            Follow
          </Flex>
        </Text>

        <Button
          variant="soft"
          onClick={() =>
            void api
              .exportDiagnostics(files.map((f) => f.name))
              .then(setExported, (e) => setError(String(e)))
          }
        >
          Export
        </Button>
      </Flex>

      {exported && (
        <Callout.Root color="jade">
          <Callout.Text>
            Written to <Code>{exported}</Code>. Read it before sending it
            anywhere — engine logs name the sites they saw.{" "}
            <Button
              size="1"
              variant="ghost"
              onClick={() => void revealItemInDir(exported)}
            >
              Show file
            </Button>
          </Callout.Text>
        </Callout.Root>
      )}

      <Card>
        <ScrollArea style={{ height: 420 }}>
          <pre
            style={{
              margin: 0,
              fontSize: "var(--font-size-1)",
              lineHeight: 1.5,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {lines.join("\n") || "(nothing matches)"}
          </pre>
        </ScrollArea>
      </Card>

      <Text size="1" color="gray">
        {lines.length} line{lines.length === 1 ? "" : "s"}
        {needle && ` matching "${filter}"`}
      </Text>
    </Flex>
  );
}
