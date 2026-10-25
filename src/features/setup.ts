import * as api from "../api";

/** The config that was running when setup was opened, to bring back if setup ends without a new
 *  one. Setup has to measure with Blockbuster off: with it on, blocked sites load and the search
 *  would conclude there is nothing to fix. */
let paused: string | undefined;

export async function pauseForSetup(status: api.EngineStatus): Promise<void> {
  // A trial run is not a config anyone chose to keep, so there is nothing to bring back.
  paused = status.ephemeral ? undefined : status.active_config;
  await api.engineStop();
}

/** Setup was left without saving anything: the config that was running comes back. */
export async function resumeAfterSetup(): Promise<void> {
  const name = paused;
  paused = undefined;
  if (name) await api.engineStart(await api.loadConfig(name));
}

/** Setup saved a new config, which replaces whatever was running. */
export const discardPaused = () => {
  paused = undefined;
};
