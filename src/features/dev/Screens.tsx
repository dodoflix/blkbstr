import { useState } from "react";
import { Field, Row, Select, Stack, Text } from "../../ui";
import { ConfigsPage } from "../ConfigsPage";
import { Dashboard } from "../Dashboard";
import { LogsPage } from "../LogsPage";
import { Onboarding } from "../Onboarding";
import { SetupWizard } from "../SetupWizard";
import { SystemPage } from "../SystemPage";
import { FakeBackend, SCENARIOS, type Scenario } from "./fake";

const noop = () => {};

const SCREENS = {
  home: { label: "Home", render: () => <Dashboard onSetup={noop} /> },
  "wizard-computer": { label: "Wizard: this computer", render: () => <SetupWizard canCancel onCancel={noop} onDone={noop} initialStep="computer" /> },
  "wizard-sites": { label: "Wizard: your sites", render: () => <SetupWizard canCancel onCancel={noop} onDone={noop} initialStep="sites" /> },
  "wizard-fix": { label: "Wizard: find a fix", render: () => <SetupWizard canCancel onCancel={noop} onDone={noop} initialStep="fix" /> },
  "wizard-keep": { label: "Wizard: keep it", render: () => <SetupWizard canCancel onCancel={noop} onDone={noop} initialStep="keep" /> },
  configs: { label: "Configs", render: () => <ConfigsPage onSetup={noop} /> },
  logs: { label: "Logs", render: () => <LogsPage active={false} /> },
  system: { label: "System", render: () => <SystemPage /> },
  onboarding: { label: "Onboarding", render: () => <Onboarding /> },
} as const;
type ScreenKey = keyof typeof SCREENS;

/** Real screens, rendered inert, over fixed fake backend state, so every situation can be looked at
 *  without producing it. For breakpoints use Viewports: media queries follow the window, not this frame. */
export function Screens() {
  const [screen, setScreen] = useState<ScreenKey>("home");
  const [scenario, setScenario] = useState<Scenario>("on");
  return (
    <Stack gap={4}>
      <Row gap={3} wrap align="flex-end">
        <Field label="Screen">
          <Select value={screen} onValueChange={setScreen} options={(Object.keys(SCREENS) as ScreenKey[]).map((k) => ({ value: k, label: SCREENS[k].label }))} />
        </Field>
        <Field label="Backend state">
          <Select value={scenario} onValueChange={setScenario} options={SCENARIOS.map((s) => ({ value: s, label: s }))} />
        </Field>
      </Row>
      <Text size="sm" color={3} as="p">
        The frame is not interactive. Pages that read logs or configs from disk still read them for real.
      </Text>
      <div className="dev-frame" inert>
        <FakeBackend key={`${screen}:${scenario}`} scenario={scenario}>{SCREENS[screen].render()}</FakeBackend>
      </div>
    </Stack>
  );
}
