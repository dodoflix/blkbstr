import { useState } from "react";
import { Button, CodeBlock, Notice, Stack, Text } from "../ui";
import * as api from "../api";
import { useBackend } from "./backend";

/** Windows installs the helper from the app, in one permission prompt; Linux has a script to run. */
export function useHelperInstall() {
  const { env, act } = useBackend();
  const [installing, setInstalling] = useState(false);
  const canInstall = env?.platform === "windows";
  const install = async () => {
    setInstalling(true);
    await act(api.serviceInstall, "The helper is installed");
    setInstalling(false);
  };
  return { canInstall, installing, install };
}

export function HelperInstall() {
  const { canInstall, installing, install } = useHelperInstall();
  if (!canInstall) return <CodeBlock code="packaging/linux/install.sh" title="Run in the project folder" />;
  return (
    <Stack gap={2} align="start">
      <Button icon="shield-check" loading={installing} onClick={() => void install()}>
        Install the helper
      </Button>
      <Text size="sm" color={3}>
        Windows asks for permission once. Only your account can talk to the helper afterwards.
      </Text>
    </Stack>
  );
}

/** After an app update the helper can keep running the old binary; installing again swaps it. */
export function HelperOutdated() {
  const { daemon } = useBackend();
  const { canInstall, installing, install } = useHelperInstall();
  if (!canInstall || !daemon?.outdated) return null;
  return (
    <Notice
      title="The background helper is out of date"
      action={
        <Button size="sm" icon="shield-check" loading={installing} onClick={() => void install()}>
          Update the helper
        </Button>
      }
    >
      It is running version {daemon.version}. Windows asks for permission once.
    </Notice>
  );
}
