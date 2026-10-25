import { createContext, useContext, useEffect, useLayoutEffect, useMemo, useState, type ReactNode } from "react";
import { ChoiceGrid, Field } from "./forms";
import { useLocalStorage } from "./hooks";
import { Stack } from "./layout";

export type ThemePreference = "system" | "light" | "dark";
export type Motion = "system" | "reduced";

type Appearance = {
  theme: ThemePreference;
  setTheme: (v: ThemePreference) => void;
  /** What `theme` resolves to right now ("system" follows the OS). */
  resolved: "light" | "dark";
  motion: Motion;
  setMotion: (v: Motion) => void;
};

const Ctx = createContext<Appearance | null>(null);

export function useAppearance(): Appearance {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useAppearance needs a ThemeProvider");
  return ctx;
}

const dark = () => window.matchMedia("(prefers-color-scheme: dark)");

/** Owns every look-and-feel preference and writes them to <html> as data attributes, which is all
 *  the stylesheet reads. Swap the storage here to sync them elsewhere. */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useLocalStorage<ThemePreference>("blkbstr.theme", "system");
  const [motion, setMotion] = useLocalStorage<Motion>("blkbstr.motion", "system");
  const [os, setOs] = useState<"light" | "dark">(() => (dark().matches ? "dark" : "light"));

  useEffect(() => {
    const mq = dark();
    const on = () => setOs(mq.matches ? "dark" : "light");
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);

  const resolved = theme === "system" ? os : theme;
  useLayoutEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = resolved;
    root.dataset.motion = motion;
  }, [resolved, motion]);

  const value = useMemo(
    () => ({ theme, setTheme, resolved, motion, setMotion }),
    [theme, setTheme, resolved, motion, setMotion],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

/** Theme and motion in one form. Used in first-run onboarding (`basic`) and in settings (`all`). */
export function AppearanceForm({ parts = "all" }: { parts?: "basic" | "all" }) {
  const a = useAppearance();
  return (
    <Stack gap={5}>
      <Field label="Theme">
        <ChoiceGrid
          aria-label="Theme"
          value={a.theme}
          onValueChange={a.setTheme}
          choices={[
            { value: "system", label: "System", description: "Match your computer", icon: "monitor" },
            { value: "light", label: "Light", description: "Bright surfaces", icon: "sun" },
            { value: "dark", label: "Dark", description: "Easy at night", icon: "moon" },
          ]}
        />
      </Field>
      {parts === "all" && (
        <Field label="Motion">
          <ChoiceGrid
            aria-label="Motion"
            min="11rem"
            value={a.motion}
            onValueChange={a.setMotion}
            choices={[
              { value: "system", label: "Standard", description: "Follow your system setting" },
              { value: "reduced", label: "Reduced", description: "No transitions or movement" },
            ]}
          />
        </Field>
      )}
    </Stack>
  );
}
