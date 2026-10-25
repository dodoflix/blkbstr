import { createContext, useContext, type ReactNode } from "react";
import { useLocalStorage } from "../ui";

type Prefs = {
  /** The Advanced layer: pids, paths, parameter files and per-step tables. Off by default. */
  advanced: boolean;
  setAdvanced: (on: boolean) => void;
  /** False until the first-run preferences screen has been completed or skipped. */
  onboarded: boolean;
  setOnboarded: (done: boolean) => void;
};

const Ctx = createContext<Prefs>({ advanced: false, setAdvanced: () => {}, onboarded: true, setOnboarded: () => {} });
export const usePrefs = () => useContext(Ctx);

export function PrefsProvider({ children }: { children: ReactNode }) {
  const [advanced, setAdvanced] = useLocalStorage("blkbstr.advanced", false);
  const [onboarded, setOnboarded] = useLocalStorage("blkbstr.onboarded", false);
  return <Ctx.Provider value={{ advanced, setAdvanced, onboarded, setOnboarded }}>{children}</Ctx.Provider>;
}
