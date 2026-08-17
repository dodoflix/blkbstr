import { useCallback, useEffect, useState } from "react";
import {
  Badge,
  Button,
  Callout,
  Card,
  Code,
  Container,
  DataList,
  Flex,
  Heading,
  Tabs,
  Text,
} from "@radix-ui/themes";
import * as api from "./api";

export default function App() {
  return (
    <Container size="2" p="5">
      <Flex direction="column" gap="4">
        <Flex align="baseline" gap="3">
          <Heading size="7">Blockbuster</Heading>
          <Text size="2" color="gray">
            zapret, with a front door
          </Text>
        </Flex>

        <Tabs.Root defaultValue="status">
          <Tabs.List>
            <Tabs.Trigger value="status">Status</Tabs.Trigger>
            <Tabs.Trigger value="configs">Configs</Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="status" mt="4">
            <StatusPanel />
          </Tabs.Content>
          <Tabs.Content value="configs" mt="4">
            <ConfigsPanel />
          </Tabs.Content>
        </Tabs.Root>
      </Flex>
    </Container>
  );
}

function StatusPanel() {
  const [daemon, setDaemon] = useState<api.DaemonInfo>();
  const [status, setStatus] = useState<api.EngineStatus>();
  const [error, setError] = useState<string>();

  const refresh = useCallback(async () => {
    const info = await api.daemonInfo();
    setDaemon(info);
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
              {daemon?.reachable ? (
                <Badge color="jade">running · v{daemon.version}</Badge>
              ) : (
                <Badge color="gray">not reachable</Badge>
              )}
            </DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Engine</DataList.Label>
            <DataList.Value>
              {status?.running ? (
                <Badge color="jade">active</Badge>
              ) : (
                <Badge color="gray">stopped</Badge>
              )}
            </DataList.Value>
          </DataList.Item>
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
