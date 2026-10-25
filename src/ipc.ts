import { invoke as tauriInvoke } from "@tauri-apps/api/core";

const inTauri = "__TAURI_INTERNALS__" in window;

/** Every backend call goes through here. Outside Tauri a dev build answers from a local mock so
 *  the UI can be built and looked at in a plain browser (see src/mock.ts for scenarios). */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.DEV && !inTauri) {
    const { mock } = await import("./mock");
    return mock<T>(cmd, args);
  }
  return tauriInvoke<T>(cmd, args);
}
