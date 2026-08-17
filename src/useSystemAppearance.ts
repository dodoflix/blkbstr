import { useSyncExternalStore } from "react";

const query = window.matchMedia("(prefers-color-scheme: dark)");

const subscribe = (onChange: () => void) => {
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
};

/**
 * Follows the OS theme. A user-facing light/dark override goes here later; until then there is
 * nothing to persist.
 */
export const useSystemAppearance = (): "light" | "dark" =>
  useSyncExternalStore(subscribe, () => (query.matches ? "dark" : "light"));
