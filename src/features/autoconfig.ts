import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import { median } from "./format";
import { usesList, withMode, type HostMode } from "./hostmode";
import { selectedHosts } from "./sites";

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** Time for the engine to start reading the queue; the rules are in place when engineStart returns. */
const SETTLE_MS = 600;

/** Trials apply the strategy to every site: behind the learned list it would do nothing to the
 *  sites being measured until they had failed enough times to be learned. */
const trial = (config: api.Config): api.Config => withMode(config, "all");

/** A candidate that filters ports the check never uses cannot change the result. */
function measurable(config: api.Config): boolean {
  return config.strategies.some(
    (s) => s.enabled && (s.filter.tcp ?? "").split(",").includes(api.PROBED_TCP_PORT),
  );
}

export type Step = {
  name: string;
  notes?: string;
  outcome: string;
  /** Undefined while the candidate is still being tried. */
  worked?: boolean;
  unblocked: number;
  cost: number;
  config: api.Config;
  /** Filled in by the measurement pass, which only runs for candidates that helped. */
  impact?: Impact;
};

/** What a strategy costs the rest of the connection, measured rather than guessed. */
export type Impact = {
  /** Median TLS handshake time to the hosts that already worked. */
  latencyMs: number;
  /** How much slower than with no strategy running. */
  deltaMs: number;
  /** Hosts that worked before this strategy and stopped once it was applied. */
  broke: string[];
};

/** Median handshake time across hosts that already worked, plus any that stopped working. Three
 *  rounds because one handshake is noisy; a host counts as broken only if it failed all three. */
async function measure(hosts: string[]): Promise<Omit<Impact, "deltaMs">> {
  const rounds: number[] = [];
  const failures = new Map<string, number>();
  const ROUNDS = 3;
  for (let i = 0; i < ROUNDS; i++) {
    const report = await api.checkReachability(hosts, api.TRIAL_TIMEOUT_MS);
    rounds.push(
      median(report.sites.filter((site) => site.verdict === "fine").map((site) => site.elapsed_ms)),
    );
    for (const site of report.sites) {
      if (site.verdict !== "fine") failures.set(site.host, (failures.get(site.host) ?? 0) + 1);
    }
  }
  return {
    latencyMs: median(rounds),
    broke: [...failures].filter(([, n]) => n === ROUNDS).map(([host]) => host),
  };
}

/** Best first: most sites through, then least disturbance, then the hand-ranked order. Partial
 *  results count: when nothing gets everything, the best one is still the best answer. */
export function rank(steps: Step[]): Step[] {
  return steps
    .filter((step) => step.unblocked > 0)
    .sort(
      (a, b) =>
        b.unblocked - a.unblocked ||
        // Breaking a site that used to work is worse than any amount of slowness.
        (a.impact?.broke.length ?? 0) - (b.impact?.broke.length ?? 0) ||
        (a.impact?.deltaMs ?? 0) - (b.impact?.deltaMs ?? 0) ||
        a.cost - b.cost,
    );
}

/** The measured cost in one phrase. Under 10ms is inside the noise of three rounds, so it reads as
 *  no difference. */
export function impactLabel(impact: Impact): string {
  const { latencyMs, deltaMs, broke } = impact;
  const speed =
    Math.abs(deltaMs) < 10
      ? `${latencyMs}ms handshake, no slower`
      : `${latencyMs}ms handshake, ${deltaMs > 0 ? "+" : ""}${deltaMs}ms`;
  return broke.length > 0 ? speed : `${speed}, nothing else broke`;
}

/** The same measurement for someone who does not know what a handshake is. */
export function plainImpact(impact: Impact): string {
  if (impact.broke.length > 0) return `Stopped ${impact.broke.length} site${impact.broke.length === 1 ? "" : "s"} that worked before`;
  if (impact.deltaMs >= 10) return `Makes connections about ${impact.deltaMs}ms slower`;
  return "No noticeable slowdown";
}

/** Tries every candidate and reports all that worked; the first success is not necessarily the one
 *  to live with. The loop lives in the UI because each step is an ordinary start, check and stop:
 *  no privileged surface, cancellation for free, and the trial run's own deadline restores the
 *  machine if the window dies. */
export function useAutoConfig() {
  const [steps, setSteps] = useState<Step[]>([]);
  const [running, setRunning] = useState(false);
  const [winner, setWinner] = useState<api.Config>();
  const [saveAs, setSaveAs] = useState("auto");
  const [note, setNote] = useState<string>();
  const [error, setError] = useState<string>();
  const [kept, setKept] = useState<string>();
  const [blockedCount, setBlockedCount] = useState(0);
  const [mode, setMode] = useState<HostMode>("auto");
  // Hosts a strategy can help with, to seed the lists so they work on the first visit.
  const seed = useRef<string[]>([]);
  const [progress, setProgress] = useState<[number, number]>([0, 0]);
  const [measuring, setMeasuring] = useState(false);
  const [done, setDone] = useState(false);
  // A ref, not state: the walk reads it inside a loop that never re-renders.
  const cancelled = useRef(false);
  // Starting from an effect can fire twice (strict mode, a quick re-render); one walk at a time.
  const walking = useRef(false);
  // Ends with the screen that owns it; otherwise it would keep starting trial runs nobody can see.
  useEffect(() => () => void (cancelled.current = true), []);

  const finish = (name: string, outcome: string, worked = false, unblocked = 0) =>
    setSteps((prev) => prev.map((step) => (step.name === name ? { ...step, outcome, worked, unblocked } : step)));

  const run = async () => {
    if (walking.current) return;
    walking.current = true;
    cancelled.current = false;
    setRunning(true);
    setDone(false);
    setMeasuring(false);
    setProgress([0, 0]);
    setSteps([]);
    setWinner(undefined);
    setNote(undefined);
    setError(undefined);
    setKept(undefined);
    try {
      // With the engine on, blocked sites load, and the walk would find nothing to fix.
      if ((await api.engineStatus()).running) {
        setError("Blockbuster is on, which hides what is blocked. Turn it off, then start the search again.");
        return;
      }
      const baseline = await api.checkReachability(await selectedHosts());
      if (!baseline.network_ok) {
        setError(
          `Even ${baseline.control.host} did not answer, so this is the network rather than a censor. Nothing can be measured until that is fixed.`,
        );
        return;
      }
      const blocked = baseline.sites.filter((site) => site.verdict !== "fine").map((site) => site.host);
      // What "did this make anything worse" is measured against; includes the control host.
      const unaffected = baseline.sites.filter((site) => site.verdict === "fine").map((site) => site.host);
      unaffected.push(baseline.control.host);
      setBlockedCount(blocked.length);
      seed.current = baseline.sites
        .filter((site) => ["tls_reset", "tls_silent", "tcp_blocked"].includes(site.verdict))
        .map((site) => site.host);
      if (blocked.length === 0) {
        setNote("Every site on the list already loads. There is nothing to work around.");
        return;
      }

      const tried: Step[] = [];
      const all = await api.autoconfigCandidates();
      const usable = all.filter(({ config }) => measurable(config));
      const skipped = all.length - usable.length;
      setProgress([0, usable.length]);
      for (const [i, { config, cost }] of usable.entries()) {
        if (cancelled.current) break;
        setProgress([i + 1, usable.length]);
        setSteps((prev) => [
          ...prev,
          { name: config.name, notes: config.notes, outcome: "trying...", unblocked: 0, cost, config },
        ]);
        let step: Step = { name: config.name, notes: config.notes, outcome: "", unblocked: 0, cost, config };
        try {
          await api.engineStart(trial(config), true);
        } catch (e) {
          // The engine refuses a config it cannot load before any rule is installed: skip it.
          finish(config.name, `the engine refused it: ${e}`);
          continue;
        }
        await sleep(SETTLE_MS);
        const report = await api.checkReachability(blocked, api.TRIAL_TIMEOUT_MS);
        // Always stop, so every candidate is measured against the same untouched machine.
        await api.engineStop();

        const unblocked = report.sites.filter((site) => site.verdict === "fine").length;
        const worked = unblocked === blocked.length;
        step = { ...step, worked, unblocked };
        tried.push(step);
        finish(
          config.name,
          worked ? `all ${blocked.length} load` : `${blocked.length - unblocked} of ${blocked.length} still blocked`,
          worked,
          unblocked,
        );
      }

      const unmeasured =
        skipped > 0
          ? ` ${skipped} that only touch other ports were skipped: the check speaks TLS on ${api.PROBED_TCP_PORT} and nothing else, so it could not tell whether they helped.`
          : "";
      // Only what helped is measured (three more checks per candidate).
      const shortlist = rank(tried);
      if (shortlist.length > 0 && !cancelled.current) {
        setMeasuring(true);
        const before = await measure(unaffected);
        for (const [i, step] of shortlist.entries()) {
          if (cancelled.current) break;
          setProgress([i + 1, shortlist.length]);
          try {
            await api.engineStart(trial(step.config), true);
            await sleep(SETTLE_MS);
            const after = await measure(unaffected);
            step.impact = { ...after, deltaMs: after.latencyMs - before.latencyMs };
          } catch {
            // A candidate that started once and will not start again is not one to offer.
            step.impact = undefined;
          } finally {
            await api.engineStop();
          }
        }
        setSteps((prev) =>
          prev.map((step) => {
            const measured = shortlist.find((s) => s.name === step.name);
            return measured ? { ...step, impact: measured.impact } : step;
          }),
        );
        setMeasuring(false);
      }

      const stopped = cancelled.current ? ` Stopped after ${tried.length} of ${usable.length}.` : "";
      const helped = rank(tried);
      if (helped.length === 0) {
        setError(
          `No strategy got anything through. The engine is stopped and the machine is back as it was.${stopped}${unmeasured}`,
        );
        return;
      }
      setWinner(helped[0].config);
      setSaveAs(helped[0].config.name);
      const best = helped[0].unblocked;
      setNote(
        best === blocked.length
          ? `${helped.filter((s) => s.worked).length} of ${tried.length} strategies got everything through. The gentlest is picked for you; any of them can be used instead.${stopped}`
          : `Nothing got all ${blocked.length} through. The best got ${best}. Pick one or try again later: a censor is not the same from one hour to the next.${stopped}${unmeasured}`,
      );
      setDone(true);
    } catch (e) {
      setError(String(e));
    } finally {
      walking.current = false;
      setMeasuring(false);
      setRunning(false);
    }
  };

  const pick = (name: string) => {
    const picked = rank(steps).find((step) => step.name === name);
    if (picked) {
      setWinner(picked.config);
      setSaveAs(picked.name);
    }
  };

  /** Restarts under the chosen name before confirming, so the saved file, running config and boot
   *  config agree. */
  const keep = async (): Promise<boolean> => {
    if (!winner) return false;
    const named = withMode({ ...winner, name: saveAs.trim() }, mode);
    try {
      // Before the start, so the list is already there when the engine reads it. A daemon that
      // cannot take it is not a reason to lose the config.
      for (const list of ["auto", "manual"] as const) {
        if (usesList(mode, list)) await api.hostsAdd(list, seed.current).catch(() => undefined);
      }
      await api.engineStart(named, true);
      await api.engineConfirm();
      await api.saveConfig(named);
      setKept(named.name);
      setWinner(undefined);
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    }
  };

  return {
    steps,
    working: rank(steps),
    running,
    measuring,
    progress,
    blockedCount,
    note,
    error,
    winner,
    saveAs,
    setSaveAs,
    mode,
    setMode,
    kept,
    done,
    run,
    cancel: () => (cancelled.current = true),
    pick,
    keep,
  };
}

export type AutoConfig = ReturnType<typeof useAutoConfig>;
