import { useCallback, useEffect, useState } from "react";
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
            <SetupPanel />
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

/** What each verdict means in one phrase, and how alarmed to look about it. */
const VERDICTS: Record<api.Verdict, { label: string; color: "jade" | "amber" | "red" }> = {
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
            zapret2 is not installed. Blockbuster drives the upstream engine and does not ship one;
            put <Code>nfqws2</Code> on <Code>PATH</Code> or in <Code>/opt/zapret2</Code>.
          </Callout.Text>
        </Callout.Root>
      )}

      {lua && !lua.supported && (
        <Callout.Root color="red">
          <Callout.Text>
            {lua.version} is too old. zapret2's strategies are Lua, so the engine will start and
            then fail to load them — LuaJIT 2.1+ or Lua 5.3+ is needed.
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
                    {env.distro.package_manager && ` \u00b7 ${env.distro.package_manager}`}
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
                Even <Code>{report.control.host}</Code> did not answer, so this is the network
                rather than a censor. Nothing below means anything until that is fixed.
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
            The privileged service is not reachable, so nothing can be started. Install it with{" "}
            <Code>packaging/linux/install.sh</Code>.
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
                Trying <Code>{status.active_config}</Code>. It reverts on its own in{" "}
                {countdown(status.revert_in_seconds)} unless you keep it — so if this broke your
                connection, doing nothing fixes it.
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
            onClick={() =>
              void act(() => api.serviceSetActive(!svc.active))
            }
          >
            {svc.active ? "Stop service" : "Start service"}
          </Button>
          <Button
            variant="soft"
            onClick={() =>
              void act(() => api.serviceSetEnabled(!svc.enabled))
            }
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
            void act(async () => api.engineStart(await api.starterConfig("default")))
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
  const [error, setError] = useState<string>();

  useEffect(() => {
    api.listConfigs().then(setNames, (e) => setError(String(e)));
  }, []);

  if (error) {
    return (
      <Callout.Root color="red">
        <Callout.Text>{error}</Callout.Text>
      </Callout.Root>
    );
  }

  return (
    <Card>
      {names.length === 0 ? (
        <Text color="gray">
          No saved configs yet. The first-run wizard will create one; until then, drop JSON files
          in the configs directory.
        </Text>
      ) : (
        <Flex direction="column" gap="1">
          {names.map((name) => (
            <Text key={name}>{name}</Text>
          ))}
        </Flex>
      )}
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
    api.listLogs().then((found) => {
      setFiles(found);
      setSelected((current) => current ?? found[0]?.name);
    }, (e) => setError(String(e)));
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
