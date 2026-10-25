import { RadioGroup } from "../ui";
import type { Config, HostList } from "../api";

/** Which sites get the strategy. Two switches in the config, four answers for a person. */
export type HostMode = "all" | "auto" | "list" | "both";

export const modeOf = (c: Config): HostMode =>
  c.auto_hostlist ? (c.manual_hostlist ? "both" : "auto") : c.manual_hostlist ? "list" : "all";

export const withMode = (c: Config, mode: HostMode): Config => ({
  ...c,
  auto_hostlist: mode === "auto" || mode === "both",
  manual_hostlist: mode === "list" || mode === "both",
});

export const usesList = (mode: HostMode, list: HostList) =>
  mode === "both" || mode === (list === "auto" ? "auto" : "list");

const MODES: { value: HostMode; label: string; description: string }[] = [
  { value: "auto", label: "Only blocked sites", description: "A site is worked around once it fails to load, and left alone again when it loads by itself." },
  { value: "list", label: "Only my list", description: "Just the sites you add to your list." },
  { value: "both", label: "My list and blocked sites", description: "Your list, plus any site that fails to load." },
  { value: "all", label: "Every site", description: "All traffic goes through the strategy. Slower, and it can break sites that worked." },
];

export function HostModePicker({ value, onChange, disabled }: { value: HostMode; onChange: (mode: HostMode) => void; disabled?: boolean }) {
  return (
    <RadioGroup
      aria-label="Which sites to work around"
      value={value}
      onValueChange={onChange}
      options={MODES.map((m) => ({ ...m, disabled }))}
    />
  );
}
