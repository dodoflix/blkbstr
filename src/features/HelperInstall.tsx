import { useState } from "react";
import { Button, Notice, Stack, Text } from "../ui";
import * as api from "../api";
import { useBackend } from "./backend";

/** The helper installs from the app in one permission prompt: UAC on Windows, polkit on Linux. */
export function useHelperInstall() {
  const { env, act } = useBackend();
  const [installing, setInstalling] = useState(false);
  const [installed, setInstalled] = useState(false);
  const canInstall = env?.platform === "windows" || env?.platform === "linux";
  const install = async () => {
    setInstalling(true);
    setInstalled(await act(api.serviceInstall, "The helper is installed"));
    setInstalling(false);
  };
  // Group membership only reaches a session at login, so on Linux the helper stays out of reach
  // until the user logs in again.
  const relogin = installed && env?.platform === "linux";
  return { canInstall, installing, install, relogin };
}

/** Shown after a Linux install while the app still cannot reach the helper. */
export function ReloginNotice() {
  const { daemon } = useBackend();
  if (daemon?.reachable) return null;
  return (
    <Notice title="Log out and back in to finish">
      Blockbuster added your account to the blkbstr group, which is what lets it talk to the helper. Linux applies group changes at the next login.
    </Notice>
  );
}

export function HelperInstall() {
  const { env } = useBackend();
  const { canInstall, installing, install, relogin } = useHelperInstall();
  if (!canInstall) return null;
  return (
    <Stack gap={3} align="start">
      <Stack gap={2} align="start">
        <Button icon="shield-check" loading={installing} onClick={() => void install()}>
          Install the helper
        </Button>
        <Text size="sm" color={3}>
          {env?.platform === "windows"
            ? "Windows asks for permission once. Only your account can talk to the helper afterwards."
            : "You will be asked for your password once. Only your account and others you add to the blkbstr group can talk to the helper afterwards."}
        </Text>
      </Stack>
      {relogin && <ReloginNotice />}
    </Stack>
  );
}

/** After an app update the helper can keep running the old binary; installing again swaps it. */
export function HelperOutdated() {
  const { daemon, env } = useBackend();
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
      It is running version {daemon.version}. {env?.platform === "windows" ? "Windows asks for permission once." : "You will be asked for your password once."}
    </Notice>
  );
}
