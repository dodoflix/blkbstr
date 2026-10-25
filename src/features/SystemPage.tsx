import {
  AppearanceForm,
  Badge,
  Button,
  Columns,
  Notice,
  KeyValue,
  Panel,
  PageHeader,
  Section,
  Sheet,
  Skeleton,
  Stack,
  StateMark,
  Switch,
  Text,
  Row,
  useConfirm,
} from "../ui";
import { useState } from "react";
import * as api from "../api";
import { useBackend } from "./backend";
import { HelperInstall } from "./HelperInstall";
import { ChooseSites } from "./SiteChoices";
import { SiteList, SiteListPlaceholder } from "./SiteList";
import { usePrefs } from "./prefs";

const found = (ok: boolean, label: string) => (
  <Row gap={2}>
    <StateMark state={ok ? "ok" : "fail"} label={ok ? "Found" : "Missing"} />
    <span>{label}</span>
  </Row>
);

/** App and engine are separate rows on purpose: the engine runs as root and updates on its own terms. */
function Updates() {
  const { env, act, refreshEnv } = useBackend();
  const [found, setFound] = useState<api.AppUpdate | null>();
  const [busy, setBusy] = useState<"check" | "install">();

  const check = () => {
    setBusy("check");
    void act(async () => setFound(await api.checkAppUpdate())).finally(() => setBusy(undefined));
  };
  const install = () => {
    setBusy("install");
    void act(() => api.installAppUpdate()).finally(() => setBusy(undefined));
  };

  return (
    <Section title="Updates" description="Checks GitHub Releases on start, every few hours and when you ask. Nothing is sent but the request itself.">
      <Sheet>
        <Stack gap={4}>
          <Row justify="space-between" gap={4}>
            <Text>
              Blockbuster{" "}
              {found === undefined ? null : found === null ? <Badge tone="ok">up to date</Badge> : <Badge tone="warn">v{found.version} available</Badge>}
            </Text>
            {found?.installable ? (
              <Button size="sm" icon="download" loading={busy === "install"} onClick={install}>
                Install and restart
              </Button>
            ) : (
              <Button size="sm" variant="soft" tone="neutral" icon="refresh" loading={busy === "check"} onClick={check}>
                Check
              </Button>
            )}
          </Row>
          {found && !found.installable && (
            <Notice tone="neutral" title="Update through your package">
              This install is managed by your package manager, or is not an AppImage. Get v{found.version} from the releases page.
            </Notice>
          )}
          {env?.engine_update && (
            <Row justify="space-between" gap={4}>
              <Text>
                zapret2 engine <Badge tone="warn">{env.engine_update} available</Badge>
              </Text>
              <Button
                size="sm"
                variant="soft"
                tone="neutral"
                icon="download"
                onClick={() => void act(async () => { await api.installEngine(); await refreshEnv(); }, "zapret2 updated. A running engine picks it up on its next start.")}
              >
                Update engine
              </Button>
            </Row>
          )}
        </Stack>
      </Sheet>
    </Section>
  );
}

export function SystemPage() {
  const { env, service, daemon, act, report, checking, check, refreshEnv } = useBackend();
  const { advanced, setAdvanced } = usePrefs();
  const confirm = useConfirm();

  return (
    <div>
      <PageHeader title="System" description="What Blockbuster found on this computer, and how it runs." />
      <Columns min="28rem" gap={5}>
        <Section
          title="This computer"
          actions={
            <Button size="sm" variant="soft" tone="neutral" icon="refresh" onClick={() => void refreshEnv()}>
              Re-detect
            </Button>
          }
        >
          <Sheet>
            <KeyValue
              items={[
                { label: "System", value: env ? `${env.distro?.pretty_name ?? env.platform ?? "unsupported"}${env.distro?.package_manager ? `, ${env.distro.package_manager}` : ""}` : <Skeleton width={140} /> },
                { label: "zapret2 engine", value: env ? found(Boolean(env.engine), env.engine ? (env.engine.version ?? "found") : "not installed") : <Skeleton width={110} />, hint: advanced ? env?.engine?.path : undefined },
                { label: "Lua", hidden: env?.platform === "windows", value: env ? found(Boolean(env.lua?.supported), env.lua ? (env.lua.version ?? "found") : "not installed") : <Skeleton width={110} />, hint: advanced ? env?.lua?.path : undefined },
                { label: "nftables", hidden: env?.platform === "windows", value: env ? found(Boolean(env.nftables), env.nftables ? (env.nftables.version ?? "found") : "not installed") : <Skeleton width={110} />, hint: advanced ? env?.nftables?.path : undefined },
                { label: "Existing install", value: env?.existing_install, hidden: !env?.existing_install },
              ]}
            />
          </Sheet>
        </Section>

        <Section title="Background helper" description={env?.platform === "windows" ? "Runs the engine in the background." : "Runs the engine and owns the firewall rules."}>
          <Sheet>
            {!service ? (
              <Skeleton width="60%" height={64} />
            ) : service.installed ? (
              <Stack gap={4}>
                <Row justify="space-between">
                  <Text>
                    Status{" "}
                    <Badge tone={service.active ? "ok" : "neutral"}>{service.active ? `running${daemon?.version ? ` v${daemon.version}` : ""}` : "stopped"}</Badge>
                  </Text>
                  <Button
                    size="sm"
                    variant="soft"
                    tone="neutral"
                    icon={service.active ? "stop" : "play"}
                    onClick={async () => {
                      if (service.active && !(await confirm({ title: "Stop the helper?", description: env?.platform === "windows" ? "Blockbuster stops working until it is started again." : "Blockbuster stops working until it is started again. You will be asked to authenticate.", confirmLabel: "Stop it", tone: "danger" }))) return;
                      void act(() => api.serviceSetActive(!service.active));
                    }}
                  >
                    {service.active ? "Stop" : "Start"}
                  </Button>
                </Row>
                <Switch
                  label="Start at boot"
                  description="Bring back your last saved config after every restart. Asks for authentication."
                  checked={service.enabled}
                  onCheckedChange={(on) => void act(() => api.serviceSetEnabled(on))}
                />
              </Stack>
            ) : (
              <Stack gap={3}>
                <Text color={2}>{service.unsupported ?? "The helper is not installed."}</Text>
                {!service.unsupported && <HelperInstall />}
              </Stack>
            )}
          </Sheet>
        </Section>

        <Section
          title="Site check"
          description="Which sites load right now."
          actions={
            <>
              <ChooseSites />
              <Button size="sm" variant="soft" tone="neutral" icon="refresh" loading={checking} onClick={() => void check()}>
                Check
              </Button>
            </>
          }
        >
          <Panel className="sites-panel">
            {report ? <SiteList report={report} /> : checking ? <SiteListPlaceholder /> : <Text color={3}>Not checked yet.</Text>}
          </Panel>
        </Section>

        <Updates />

        <Section title="Appearance">
          <Sheet>
            <Stack gap={5}>
              <AppearanceForm />
              <Switch label="Show details" description="Process ids, file paths, raw parameter files and every attempt, everywhere they exist." checked={advanced} onCheckedChange={setAdvanced} />
            </Stack>
          </Sheet>
        </Section>
      </Columns>
    </div>
  );
}
