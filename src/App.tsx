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
            <Tabs.Trigger value="configs">Configs</Tabs.Trigger>
            <Tabs.Trigger value="logs">Logs</Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="status" mt="4">
            <StatusPanel />
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
