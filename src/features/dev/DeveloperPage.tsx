import { Gallery } from "../../ui/Gallery";
import { Field, PageHeader, Row, SegmentedControl, Sheet, Tabs, Text, useAppearance } from "../../ui";
import { Screens } from "./Screens";
import { Tokens } from "./Tokens";
import { Viewports } from "./Viewports";

/** Dev builds only (see App.tsx). A place to look at everything the UI is made of, in both themes and
 *  motion settings, for regression. */
export default function DeveloperPage() {
  const a = useAppearance();
  return (
    <div>
      <PageHeader title="Developer" description="Every component, token, screen and window size. Not part of release builds." />
      <Sheet padding="md" className="dev-bar">
        <Row gap={5} wrap align="flex-end">
          <Field label="Theme">
            <SegmentedControl aria-label="Theme" value={a.theme} onValueChange={a.setTheme} options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]} />
          </Field>
          <Field label="Motion">
            <SegmentedControl aria-label="Motion" value={a.motion} onValueChange={a.setMotion} options={[{ value: "system", label: "Standard" }, { value: "reduced", label: "Reduced" }]} />
          </Field>
        </Row>
      </Sheet>
      <div className="dev-tabs">
        <Tabs
          items={[
            { value: "components", label: "Components", icon: "layers", content: <Gallery /> },
            { value: "tokens", label: "Tokens", icon: "sliders", content: <Tokens /> },
            { value: "screens", label: "Screens", icon: "home", content: <Screens /> },
            { value: "viewports", label: "Window sizes", icon: "monitor", content: <Viewports /> },
          ]}
        />
      </div>
      <Text size="sm" color={3} as="p">
        Regression pass: each tab in light and dark, reduced motion.
      </Text>
    </div>
  );
}
