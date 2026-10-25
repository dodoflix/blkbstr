import { AppearanceForm, Button, ChoiceGrid, Field, Heading, Row, Sheet, Stack, Text } from "../ui";
import { usePrefs } from "./prefs";

/** First run: choose look and detail level before any setup. Everything here is also in System. */
export function Onboarding() {
  const { advanced, setAdvanced, setOnboarded } = usePrefs();
  return (
    <Sheet padding="lg" className="onboarding">
      <Stack gap={5}>
        <Stack gap={1}>
          <Heading level={1} size="2xl">
            Welcome to Blockbuster
          </Heading>
          <Text as="p" size="lg" color={2}>
            Choose how it looks and how much detail you want. You can change all of this later in System.
          </Text>
        </Stack>

        <AppearanceForm parts="basic" />

        <Field label="Detail">
          <ChoiceGrid
            aria-label="Detail level"
            min="14rem"
            value={advanced ? "advanced" : "simple"}
            onValueChange={(v) => setAdvanced(v === "advanced")}
            choices={[
              { value: "simple", label: "Simple", description: "Plain words and one clear next step", icon: "wand" },
              { value: "advanced", label: "Advanced", description: "Process ids, file paths, parameter files, every attempt", icon: "sliders" },
            ]}
          />
        </Field>

        <Row gap={2} wrap>
          <Button size="lg" iconRight="arrow-right" onClick={() => setOnboarded(true)}>
            Continue to setup
          </Button>
        </Row>
      </Stack>
    </Sheet>
  );
}
