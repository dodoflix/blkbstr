import { useEffect, useRef, useState } from "react";
import * as api from "../api";
import { median } from "./format";

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms));

/** The rules are already installed when engineStart returns; this is only for the engine to be
 *  reading the queue. Measured at well under 250ms, so this is mostly margin. */
const SETTLE_MS = 600;

/** A candidate that filters ports the check never speaks on cannot change what the check measures,
 *  so trying it costs a full timeout to learn nothing. */
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

/** Handshake time across hosts that already worked, and anything that stopped working.
 *
 *  Three rounds: one handshake is noisy enough to invent a difference that is not there. A host
 *  counts as broken only if it failed every round, so a single flap does not condemn a strategy. */
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

/** Best first: more sites through wins, then the strategy that disturbs traffic least, then the
 *  earlier one, which is the hand-ranked order.
 *
 *  Includes strategies that only got some of the sites through. When nothing gets everything, the
 *  one that gets most of it is still the best answer available, and hiding it helps nobody. */
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

/** The measured cost of a strategy in one phrase. A handshake difference under 10ms is inside the
 *  noise of three rounds, so it is reported as no difference rather than as a small one. */
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

/** Tries every candidate and reports all of them that worked.
 *
 *  It does not stop at the first success: the first strategy that gets through is not necessarily
 *  the one to live with, and which trade-off is acceptable is the user's call, not a heuristic's.
 *
 *  The loop lives here rather than in the daemon because every step is already an ordinary
 *  start / check / stop: driving it from the UI costs no privileged surface, makes progress and
 *  cancellation free, and if this window dies mid-walk the trial run's own deadline puts the
 *  machine back. */
export function useAutoConfig() {
  const [steps, setSteps] = useState<Step[]>([]);
  const [running, setRunning] = useState(false);
  const [winner, setWinner] = useState<api.Config>();
  const [saveAs, setSaveAs] = useState("auto");
  const [note, setNote] = useState<string>();
  const [error, setError] = useState<string>();
  const [kept, setKept] = useState<string>();
  const [blockedCount, setBlockedCount] = useState(0);
  const [progress, setProgress] = useState<[number, number]>([0, 0]);
  const [measuring, setMeasuring] = useState(false);
  const [done, setDone] = useState(false);
  // A ref, not state: the walk reads it inside a loop that never re-renders.
  const cancelled = useRef(false);
  // A walk with nothing watching it would keep starting trial runs the user can neither see nor
  // stop, so it ends with the screen that owns it.
  useEffect(() => () => void (cancelled.current = true), []);

  const finish = (name: string, outcome: string, worked = false, unblocked = 0) =>
    setSteps((prev) => prev.map((step) => (step.name === name ? { ...step, outcome, worked, unblocked } : step)));

  const run = async () => {
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
      const baseline = await api.checkReachability();
      if (!baseline.network_ok) {
        setError(
          `Even ${baseline.control.host} did not answer, so this is the network rather than a censor. Nothing can be measured until that is fixed.`,
        );
        return;
      }
      const blocked = baseline.sites.filter((site) => site.verdict !== "fine").map((site) => site.host);
      // The hosts a strategy is not supposed to change. They are what "did this make anything
      // worse" is measured against, so the control host is in there even if nothing else is.
      const unaffected = baseline.sites.filter((site) => site.verdict === "fine").map((site) => site.host);
      unaffected.push(baseline.control.host);
      setBlockedCount(blocked.length);
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
          await api.engineStart(config, true);
        } catch (e) {
          // The engine refuses a config it cannot load before any rule is installed. That is a
          // candidate to skip, not a reason to stop.
          finish(config.name, `the engine refused it: ${e}`);
          continue;
        }
        await sleep(SETTLE_MS);
        const report = await api.checkReachability(blocked, api.TRIAL_TIMEOUT_MS);
        // Always stop: every candidate is measured against the same untouched machine, and the
        // one the user settles on is started again by name when they keep it.
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
      // Only what helped gets measured: it is three more checks per candidate, and a strategy that
      // did not get anything through is not a choice however fast it is.
      const shortlist = rank(tried);
      if (shortlist.length > 0 && !cancelled.current) {
        setMeasuring(true);
        const before = await measure(unaffected);
        for (const [i, step] of shortlist.entries()) {
          if (cancelled.current) break;
          setProgress([i + 1, shortlist.length]);
          try {
            await api.engineStart(step.config, true);
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

  /** Restarts under the chosen name before confirming, so the saved file, the running config and
   *  the one that comes back at boot all say the same thing. */
  const keep = async (): Promise<boolean> => {
    if (!winner) return false;
    const named = { ...winner, name: saveAs.trim() };
    try {
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
    kept,
    done,
    run,
    cancel: () => (cancelled.current = true),
    pick,
    keep,
  };
}

export type AutoConfig = ReturnType<typeof useAutoConfig>;
