import { useEffect, useRef } from "react";
import {
  Badge,
  Button,
  CodeBlock,
  Columns,
  Heading,
  KeyValue,
  Notice,
  Panel,
  Row,
  Select,
  Sheet,
  Skeleton,
  Stack,
  Stamp,
  Text,
  useLocalStorage,
  useNow,
} from "../ui";
import * as api from "../api";
import { useBackend } from "./backend";
import { countdown, plural, uptime } from "./format";
import { usePrefs } from "./prefs";
import { SiteList } from "./SiteList";

const ROW_PLACEHOLDERS = 6;

function SitesPlaceholder() {
  return (
    <Stack gap={0} aria-hidden>
      {Array.from({ length: ROW_PLACEHOLDERS }, (_, i) => (
        <div className="site-skeleton" key={i}>
          <Skeleton width={18} height={18} />
          <Skeleton width={`${28 + ((i * 11) % 22)}%`} height={14} />
        </div>
      ))}
    </Stack>
  );
}

/** Same footprint as the real Home, so nothing moves when the daemon answers. */
export function HomePlaceholder() {
  return (
    <Columns min="26rem" gap={5}>
      <Sheet padding="lg" className="home-sheet">
        <Stack gap={3}>
          <Skeleton width="55%" height={36} />
          <Skeleton width="30%" height={18} />
          <Skeleton width={120} height={44} />
        </Stack>
      </Sheet>
      <Panel title="Your sites" description="&nbsp;">
        <SitesPlaceholder />
      </Panel>
    </Columns>
  );
}

/** Top of Home: one sentence for what the machine is doing, one button to change it. */
function StatusSheet({ onSetup }: { onSetup: () => void }) {
  const { status, configs, act, env, report } = useBackend();
  const { advanced } = usePrefs();
  const now = useNow(30_000);
  const [last, setLast] = useLocalStorage<string | undefined>("blkbstr.lastConfig", undefined);
  const chosen = last && configs.includes(last) ? last : configs[0];
  const on = Boolean(status?.running);
  const trial = status?.revert_in_seconds !== undefined;
  const missing = !env?.engine;
  const stillBlocked = report?.sites.filter((s) => s.verdict !== "fine").length;

  const turnOn = () =>
    act(async () => {
      if (!chosen) return;
      await api.engineStart(await api.loadConfig(chosen));
      setLast(chosen);
    });

  let headline: React.ReactNode;
  let sub: string;
  if (missing) {
    headline = <>Can't start yet.</>;
    sub = "zapret2 is not installed on this computer.";
  } else if (trial) {
    headline = (
      <>
        <span className="home-state__word">Trying</span> {status?.active_config}.
      </>
    );
    sub = `Reverts in ${countdown(status!.revert_in_seconds!)} unless you keep it.`;
  } else if (on) {
    headline = (
      <>
        <span className="home-state__word" data-on>
          On.
        </span>{" "}
        {stillBlocked === undefined
          ? "Checking your sites."
          : stillBlocked === 0
            ? "Blocked sites load."
            : `${plural(stillBlocked, "site")} still blocked.`}
      </>
    );
    sub = `Running ${status?.active_config}${status?.started_at ? ` for ${uptime(status.started_at, now)}` : ""}.`;
  } else {
    headline = (
      <>
        <span className="home-state__word">Off.</span> Your connection is untouched.
      </>
    );
    sub = chosen ? `${chosen} is ready.` : "No saved config yet.";
  }

  return (
    <Sheet padding="lg" className="home-sheet">
      <Stack gap={5}>
        <Row justify="space-between" align="flex-start" gap={5}>
          <Stack gap={2}>
            <Heading level={1} size="3xl" className="home-state">
              {headline}
            </Heading>
            <Text size="lg" color={2} className="home-sub">
              {sub}
            </Text>
          </Stack>
          {trial && <Stamp tone="warn" className="home-stamp">On trial</Stamp>}
        </Row>

        <Row gap={3} wrap>
          {missing ? (
            <Button size="lg" icon="wand" onClick={onSetup}>
              Fix this in setup
            </Button>
          ) : trial ? (
            <>
              <Button size="lg" icon="check" onClick={() => void act(api.engineConfirm, "Kept. It will come back after a reboot.")}>
                Keep it
              </Button>
              <Button size="lg" variant="soft" tone="danger" icon="undo" onClick={() => void act(api.engineStop)}>
                Undo now
              </Button>
              <Text size="sm" color={3} className="home-hint">
                If this broke your connection, doing nothing fixes it.
              </Text>
            </>
          ) : on ? (
            <Button size="lg" tone="neutral" variant="soft" icon="power" onClick={() => void act(api.engineStop)}>
              Turn off
            </Button>
          ) : (
            <Button size="lg" icon="power" disabled={!chosen} onClick={() => void turnOn()}>
              Turn on
            </Button>
          )}
          {advanced && !on && configs.length > 1 && (
            <Select aria-label="Config to run" size="lg" value={chosen} onValueChange={setLast} options={configs.map((c) => ({ value: c, label: c }))} />
          )}
        </Row>
      </Stack>
    </Sheet>
  );
}

function ServiceDown() {
  const { daemon, refresh } = useBackend();
  return (
    <Sheet padding="lg" className="home-sheet">
      <Stack gap={4}>
        <Heading level={1} size="3xl" className="home-state">
          The background helper isn't running.
        </Heading>
        <Text as="p" size="lg" color={2}>
          Blockbuster changes firewall rules through a small service. Install it once, then check again.
        </Text>
        <CodeBlock code="packaging/linux/install.sh" title="Run in the project folder" />
        <Row gap={3}>
          <Button icon="refresh" onClick={() => void refresh()}>
            Check again
          </Button>
        </Row>
        {daemon?.problem && (
          <Text size="sm" color={3} mono>
            {daemon.problem}
          </Text>
        )}
      </Stack>
    </Sheet>
  );
}

export function Dashboard({ onSetup }: { onSetup: () => void }) {
  const { status, daemon, report, checking, check } = useBackend();
  const { advanced } = usePrefs();
  const reachable = daemon?.reachable;
  const running = status?.running;

  useEffect(() => {
    if (reachable && !report) void check();
  }, [reachable, report, check]);

  // A site list that predates the engine being toggled is stale; re-test when the state flips.
  const wasRunning = useRef(running);
  useEffect(() => {
    if (wasRunning.current !== running && reachable) void check();
    wasRunning.current = running;
  }, [running, reachable, check]);

  if (!reachable) return <ServiceDown />;

  const blockedSites = report?.sites.filter((s) => s.verdict !== "fine").length ?? 0;

  return (
    <Columns min="26rem" gap={5}>
      <StatusSheet onSetup={onSetup} />
        {advanced && (
          <Panel title="Details">
            <KeyValue
              items={[
                { label: "Service", value: <Badge tone="ok">running v{daemon?.version}</Badge> },
                { label: "Engine", value: running ? <Badge tone="ok">active</Badge> : <Badge>stopped</Badge> },
                { label: "Config", value: status?.active_config ?? "none", hidden: !status?.active_config },
                { label: "Process", value: <Text mono>pid {status?.pid}</Text>, hidden: !status?.pid },
                { label: "Engine version", value: <Text mono>{status?.engine_version}</Text>, hidden: !status?.engine_version },
                { label: "Last error", value: <Text color="danger">{status?.last_error}</Text>, hidden: !status?.last_error },
              ]}
            />
          </Panel>
        )}

      <Panel
        title="Your sites"
        description={
          report
            ? blockedSites === 0
              ? "Everything on the list loads."
              : `${plural(blockedSites, "site")} of ${report.sites.length} still ${blockedSites === 1 ? "does" : "do"} not load.`
            : "Checking which sites load"
        }
        actions={
          <>
            <Button variant="soft" tone="neutral" size="sm" icon="refresh" loading={checking} onClick={() => void check()}>
              Check again
            </Button>
            <Button variant="soft" tone="neutral" size="sm" icon="wand" onClick={onSetup}>
              Run setup again
            </Button>
          </>
        }
      >
        {report ? <SiteList report={report} /> : <SitesPlaceholder />}
        {report && !report.network_ok && (
          <Notice tone="warn" title="The network looks down">
            Even {report.control.host} did not answer, so these results do not mean anything until that is fixed.
          </Notice>
        )}
      </Panel>
    </Columns>
  );
}
