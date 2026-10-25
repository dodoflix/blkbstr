export type Tone = "neutral" | "accent" | "ok" | "warn" | "danger";
export type Size = "sm" | "md" | "lg";
export type Variant = "solid" | "soft" | "outline" | "ghost";
/** What a mark, row or step is doing, independent of colour. */
export type State = "ok" | "idle" | "busy" | "fail" | "warn";

export const STATE_TONE: Record<State, Tone> = {
  ok: "ok",
  idle: "neutral",
  busy: "accent",
  fail: "danger",
  warn: "warn",
};
