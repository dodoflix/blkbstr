import { useEffect, useState, type ReactNode } from "react";
import {
  Badge,
  Button,
  CodeBlock,
  Disclosure,
  Heading,
  Notice,
  Progress,
  RadioGroup,
  Row,
  RowList,
  Sheet,
  Stack,
  Stamp,
  StateMark,
  Table,
  Text,
  TextField,
  Field,
  Steps,
  type State,
  useMediaQuery,
  type Step,
  useToast,
} from "../ui";
import * as api from "../api";
import { useHelperInstall } from "./HelperInstall";
import { useBackend } from "./backend";
import { impactLabel, plainImpact, useAutoConfig, type AutoConfig } from "./autoconfig";
import { plural } from "./format";
import { usePrefs } from "./prefs";
import { SiteList } from "./SiteList";

const STEPS: (Step & { title: string; lead: string })[] = [
  { id: "computer", label: "This computer", title: "Check this computer", lead: "Blockbuster needs a few things in place before it can do anything." },
  { id: "sites", label: "Your sites", title: "See what is blocked", lead: "A quick test of commonly blocked sites tells us how your network is blocking them." },
  { id: "fix", label: "Find a fix", title: "Find what works", lead: "Blockbuster tries one strategy after another and measures which ones get through." },
  { id: "keep", label: "Keep it", title: "Save and turn on", lead: "Name it, and it comes back after every reboot." },
];

/** Sticky so the way forward never scrolls out of view under a long list. */
function StepActions({ children }: { children: ReactNode }) {
  return (
    <div className="wizard__actions">
      <Row gap={2} wrap>
        {children}
      </Row>
    </div>
  );
}

function requirement({ state, title, detail, action }: { state: State; title: string; detail: ReactNode; action?: ReactNode }) {
  return {
    id: title,
    leading: <StateMark state={state} size={20} label={state === "ok" ? "Ready" : "Missing"} />,
    title,
    subtitle: detail,
    trailing: action,
  };
}

function ComputerStep({ onNext }: { onNext: () => void }) {
  const { env, daemon, refresh, refreshEnv, refreshConfigs } = useBackend();
  const { advanced } = usePrefs();
  const toast = useToast();
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installError, setInstallError] = useState<string>();
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string>();
  const [imported, setImported] = useState<api.LegacyImport>();
  const helper = useHelperInstall();
  if (!env || !daemon) return <Progress indeterminate label="Looking at this computer" />;

  const windows = env.platform === "windows";
  const canInstall = env.platform === "linux" || windows;

  async function install() {
    setInstalling(true);
    setInstallError(undefined);
    try {
      await api.installEngine();
      await refreshEnv();
      toast({ title: "zapret2 installed", tone: "ok" });
    } catch (e) {
      setInstallError(String(e));
    } finally {
      setInstalling(false);
    }
  }

  async function importLegacy() {
    setImporting(true);
    setImportError(undefined);
    try {
      setImported(await api.importLegacy());
      await refreshConfigs();
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  }

  const rows = [
    requirement({
      state: daemon.reachable ? "ok" : "fail",
      title: "Background helper",
      detail: daemon.reachable ? `Running${advanced ? `, version ${daemon.version}` : ""}` : "Not running. Install it once, then check again.",
      action:
        !daemon.reachable && windows ? (
          <Button size="sm" icon="shield-check" loading={helper.installing} onClick={() => void helper.install()}>
            Install
          </Button>
        ) : undefined,
    }),
    requirement({
      state: env.engine ? "ok" : installing ? "busy" : "fail",
      title: "zapret2 engine",
      detail: env.engine
        ? advanced ? `${env.engine.version ?? "found"} at ${env.engine.path}` : "Installed"
        : installing
          ? `Downloading zapret2, then asking for ${windows ? "permission" : "your password"} to install it.`
          : !canInstall
            ? "Not installed. Put nfqws2 on your PATH or in /opt/zapret2."
            : daemon.reachable
              ? windows
                ? "Not installed. Blockbuster downloads the official release and installs it in Program Files. Windows will ask for permission."
                : "Not installed. Blockbuster downloads the official release and installs it in /opt/zapret2. You will be asked for your password."
              : "Not installed. Install the background helper first, then Blockbuster can install this for you.",
      action:
        !env.engine && canInstall ? (
          <Button size="sm" icon="download" loading={installing} disabled={!daemon.reachable} onClick={() => void install()}>
            {installError ? "Try again" : "Install"}
          </Button>
        ) : undefined,
    }),
    // winws2 carries its own Lua and brings its own packet driver, so neither is something to check for.
    ...(windows ? [] : [requirement({
      state: env.lua?.supported ? "ok" : "fail",
      title: "Lua",
      detail: !env.lua ? "Not installed. LuaJIT 2.1+ or Lua 5.3+ is needed." : env.lua.supported ? (advanced ? `${env.lua.version} at ${env.lua.path}` : "Installed") : `${env.lua.version} is too old. LuaJIT 2.1+ or Lua 5.3+ is needed.`,
    }),
    requirement({
      state: env.nftables ? "ok" : "fail",
      title: "Firewall tools (nftables)",
      detail: env.nftables ? (advanced ? `${env.nftables.version ?? "found"} at ${env.nftables.path}` : "Installed") : "Not installed. Blockbuster uses nftables to redirect traffic to the engine.",
    })]),
  ];
  const ready = daemon.reachable && Boolean(env.engine) && (windows || (Boolean(env.lua?.supported) && Boolean(env.nftables)));

  return (
    <Stack gap={5}>
      <RowList label="Requirements" items={rows} />
      {!daemon.reachable && !windows && <CodeBlock code="packaging/linux/install.sh" title="Install the helper" />}
      {installError && (
        <Notice tone="danger" title="The engine did not install">
          {installError}
        </Notice>
      )}
      {env.existing_install && !imported && (
        <Notice
          tone="neutral"
          title="An existing zapret2 install was found"
          action={
            <Button size="sm" variant="soft" tone="neutral" icon="download" loading={importing} onClick={() => void importLegacy()}>
              Import as Legacy
            </Button>
          }
        >
          <Text mono>{env.existing_install}</Text> stays as it is. Importing copies its config aside first and saves your strategies as a config named Legacy.
        </Notice>
      )}
      {importError && (
        <Notice tone="danger" title="That install could not be imported">
          {importError}
        </Notice>
      )}
      {imported && (
        <Notice tone="ok" title={`Imported as ${imported.name}`}>
          Your original config is backed up at <Text mono>{imported.backup}</Text>.
          {imported.skipped.length > 0 && (
            <Disclosure summary={`${plural(imported.skipped.length, "option")} not imported`}>
              <CodeBlock code={imported.skipped.join("\n")} copyable={false} />
            </Disclosure>
          )}
        </Notice>
      )}
      {advanced && env.distro && (
        <Text size="sm" color={3}>
          {env.distro.pretty_name ?? env.distro.id}
          {env.distro.package_manager && `, package manager: ${env.distro.package_manager}`}
        </Text>
      )}
      <StepActions>
        <Button disabled={!ready} iconRight="arrow-right" onClick={onNext}>
          Continue
        </Button>
        <Button
          variant="soft"
          tone="neutral"
          icon="refresh"
          loading={checking}
          onClick={async () => {
            setChecking(true);
            await Promise.all([refresh(), refreshEnv()]);
            setChecking(false);
          }}
        >
          Check again
        </Button>
      </StepActions>
    </Stack>
  );
}

function SitesStep({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const { report, checking, check } = useBackend();
  useEffect(() => {
    void check();
  }, [check]);

  const blocked = report?.sites.filter((s) => s.verdict !== "fine").length ?? 0;
  return (
    <Stack gap={5}>
      {checking || !report ? (
        <Progress indeterminate label="Testing sites" />
      ) : (
        <>
          {!report.network_ok && (
            <Notice tone="warn" title="The network looks down">
              Even {report.control.host} did not answer. Fix the connection first, then test again.
            </Notice>
          )}
          {report.network_ok && blocked === 0 && (
            <Notice tone="ok" title="Everything on the list loads already">
              There is nothing to work around right now. You can still continue and save a config for later.
            </Notice>
          )}
          {report.network_ok && blocked > 0 && (
            <Notice tone="warn" title={`${blocked} of ${report.sites.length} sites are blocked`}>
              That is what the next step will try to fix.
            </Notice>
          )}
          <SiteList report={report} />
        </>
      )}
      <StepActions>
        <Button variant="soft" tone="neutral" icon="arrow-left" onClick={onBack}>
          Back
        </Button>
        <Button disabled={checking || !report?.network_ok} iconRight="arrow-right" onClick={onNext}>
          Continue
        </Button>
        <Button variant="soft" tone="neutral" icon="refresh" loading={checking} onClick={() => void check()}>
          Test again
        </Button>
      </StepActions>
    </Stack>
  );
}

function FixStep({ ac, onNext, onBack }: { ac: AutoConfig; onNext: () => void; onBack: () => void }) {
  const { advanced } = usePrefs();
  const [cur, total] = ac.progress;
  const latest = ac.steps[ac.steps.length - 1];
  const started = ac.running || ac.steps.length > 0 || ac.error || ac.note;

  return (
    <Stack gap={5}>
      {!started && (
        <Stack gap={4}>
          <Text as="p" color={2}>
            This takes a few minutes. Your connection may blink while strategies are tried. Nothing is kept until you say so, and it all undoes itself if this window closes.
          </Text>
          <Row>
            <Button size="lg" icon="play" onClick={() => void ac.run()}>
              Start the search
            </Button>
          </Row>
        </Stack>
      )}

      {ac.running && (
        <Stack gap={3}>
          <Progress
            label="Search progress"
            value={total ? cur : undefined}
            max={total || 1}
            indeterminate={!total}
          />
          <Row justify="space-between" gap={3}>
            <Text color={2}>
              {ac.measuring ? "Measuring the cost of the ones that worked" : "Trying strategies"}
              {total > 0 && ` (${cur} of ${total})`}
              {advanced && latest && !ac.measuring && <Text mono color={3}>{`  ${latest.name}`}</Text>}
            </Text>
            <Button size="sm" variant="soft" tone="neutral" icon="stop" onClick={ac.cancel}>
              Stop and use what we have
            </Button>
          </Row>
          {ac.working.length > 0 && (
            <Text size="sm" color="ok">
              {plural(ac.working.length, "strategy", "strategies")} already got through.
            </Text>
          )}
        </Stack>
      )}

      {ac.error && <Notice tone="warn" title="No answer yet">{ac.error}</Notice>}
      {ac.note && ac.done && <Notice tone="ok">{ac.note}</Notice>}
      {ac.note && !ac.done && !ac.error && <Notice tone="ok">{ac.note}</Notice>}

      {!ac.running && ac.working.length > 0 && (
        <Stack gap={3}>
          <Heading level={3} size="lg">
            Choose how to unblock
          </Heading>
          <RadioGroup
            variant="cards"
            aria-label="Strategies that worked"
            value={ac.winner?.name}
            onValueChange={ac.pick}
            options={ac.working.map((step, i) => ({
              value: step.name,
              label: (
                <Row gap={2} wrap>
                  <span>{step.name}</span>
                  {i === 0 && <Badge tone="ok">gentlest</Badge>}
                  {!step.worked && (
                    <Badge tone="warn">
                      {step.unblocked} of {ac.blockedCount}
                    </Badge>
                  )}
                  {step.impact && step.impact.broke.length > 0 && <Badge tone="danger">broke {step.impact.broke.join(", ")}</Badge>}
                </Row>
              ),
              description: step.impact ? (advanced ? impactLabel(step.impact) : plainImpact(step.impact)) : advanced ? `disturbance ${step.cost}` : undefined,
              content: advanced && step.notes ? <Text size="sm" color={3}>{step.notes}</Text> : undefined,
            }))}
          />
        </Stack>
      )}

      {advanced && ac.steps.length > 0 && (
        <Disclosure summary={`All ${ac.steps.length} attempts`}>
          <Table
            caption="Every strategy tried"
            dense
            rows={ac.steps}
            rowKey={(s) => s.name}
            columns={[
              { key: "name", header: "Strategy", cell: (s) => s.name, mono: true, sortValue: (s) => s.name },
              { key: "outcome", header: "Result", cell: (s) => <Badge tone={s.worked ? "ok" : "neutral"}>{s.outcome}</Badge> },
              { key: "notes", header: "What it does", cell: (s) => <Text size="sm" color={2}>{s.notes}</Text> },
            ]}
          />
        </Disclosure>
      )}

      <StepActions>
        <Button variant="soft" tone="neutral" icon="arrow-left" disabled={ac.running} onClick={onBack}>
          Back
        </Button>
        <Button disabled={ac.running || !ac.winner} iconRight="arrow-right" onClick={onNext}>
          Continue
        </Button>
        {started && !ac.running && (
          <Button variant="soft" tone="neutral" icon="refresh" onClick={() => void ac.run()}>
            Search again
          </Button>
        )}
      </StepActions>
    </Stack>
  );
}

function KeepStep({ ac, onBack, onDone }: { ac: AutoConfig; onBack: () => void; onDone: () => void }) {
  const { refresh, refreshConfigs } = useBackend();
  const [saving, setSaving] = useState(false);

  if (ac.kept) {
    return (
      <Stack gap={5} align="flex-start">
        <Stamp tone="ok" size="lg" animate>
          Kept
        </Stamp>
        <Text as="p" size="lg" color={2}>
          <strong>{ac.kept}</strong> is running and will come back after a reboot.
        </Text>
        <Button iconRight="arrow-right" onClick={onDone}>
          Go to Home
        </Button>
      </Stack>
    );
  }

  return (
    <Stack gap={5}>
      {!ac.winner && (
        <Notice tone="warn" title="Nothing to save yet">
          Find a fix first, then come back here.
        </Notice>
      )}
      {ac.error && <Notice tone="danger" title="That did not save">{ac.error}</Notice>}
      <Field label="Name" hint="Letters, numbers and dashes. You can run it again from Configs.">
        <TextField className="wizard__name" value={ac.saveAs} onChange={(e) => ac.setSaveAs(e.currentTarget.value)} />
      </Field>
      <Text size="sm" color={3}>
        Until you save, the trial undoes itself.
      </Text>
      <StepActions>
        <Button variant="soft" tone="neutral" icon="arrow-left" onClick={onBack}>
          Back
        </Button>
        <Button
          icon="shield-check"
          loading={saving}
          disabled={!ac.saveAs.trim() || !ac.winner}
          onClick={async () => {
            setSaving(true);
            await ac.keep();
            await Promise.all([refresh(), refreshConfigs()]);
            setSaving(false);
          }}
        >
          Save and turn on
        </Button>
      </StepActions>
    </Stack>
  );
}

export function SetupWizard({
  onDone,
  onCancel,
  canCancel,
  initialStep = "computer",
}: {
  onDone: () => void;
  onCancel: () => void;
  canCancel: boolean;
  initialStep?: string;
}) {
  const [step, setStep] = useState(initialStep);
  const ac = useAutoConfig();
  const at = STEPS.find((s) => s.id === step)!;
  const go = (id: string) => setStep(id);
  const narrow = useMediaQuery("(max-width: 860px)");

  return (
    <Sheet padding="none" className="wizard">
      <aside className="wizard__rail">
        <Steps steps={STEPS} orientation={narrow ? "horizontal" : "vertical"} current={step} reachable={STEPS.length - 1} onSelect={ac.running ? undefined : go} />
        {canCancel && (
          <Button variant="soft" tone="neutral" size="sm" icon="x" disabled={ac.running} onClick={onCancel}>
            Leave setup
          </Button>
        )}
      </aside>
      <div className="wizard__main">
        <Stack gap={1} className="wizard__head">
          <Heading level={1} size="2xl">
            {at.title}
          </Heading>
          <Text color={2} as="p" size="lg">
            {at.lead}
          </Text>
        </Stack>
        <div className="wizard__body" key={step}>
          {step === "computer" && <ComputerStep onNext={() => go("sites")} />}
          {step === "sites" && <SitesStep onBack={() => go("computer")} onNext={() => go("fix")} />}
          {step === "fix" && <FixStep ac={ac} onBack={() => go("sites")} onNext={() => go("keep")} />}
          {step === "keep" && <KeepStep ac={ac} onBack={() => go("fix")} onDone={onDone} />}
        </div>
      </div>
    </Sheet>
  );
}
