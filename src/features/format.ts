import type { Verdict } from "../api";
import type { State } from "../ui";

/** Coarse on purpose: "3h" is what a user wants from an uptime, not "3h 07m 41s". */
export function uptime(since: number, now = Date.now()): string {
  const seconds = Math.max(0, Math.floor(now / 1000) - since);
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return `${Math.floor(seconds / 86400)}d`;
}

/** Coarse, because the status is polled every few seconds and a per-second countdown would
 *  visibly stutter. Approximate is honest here; the daemon holds the real deadline. */
export function countdown(seconds: number): string {
  if (seconds >= 60) return `about ${Math.round(seconds / 60)} min`;
  return `${seconds}s`;
}

export const median = (xs: number[]) =>
  xs.length ? [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)] : 0;

export const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

export const VERDICTS: Record<Verdict, { label: string; state: State }> = {
  fine: { label: "Loads", state: "ok" },
  dns_failed: { label: "DNS did not answer", state: "warn" },
  dns_poisoned: { label: "Fake DNS answer", state: "fail" },
  tcp_blocked: { label: "Connection blocked", state: "fail" },
  tls_reset: { label: "Reset after hello", state: "fail" },
  tls_silent: { label: "Silently dropped", state: "fail" },
  bad_host: { label: "Bad hostname", state: "warn" },
};
