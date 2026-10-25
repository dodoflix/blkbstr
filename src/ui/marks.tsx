import type { ReactNode } from "react";
import { cx } from "./cx";
import { STATE_TONE, type State } from "./types";

/** State drawn as a shape first and a colour second. One square family, told apart by fill and
 *  glyph: solid with a check holds, hollow is idle, dashed is in progress, solid with a cross is
 *  blocked, a square inside a square is attention. */
export function StateMark({ state, size = 16, label }: { state: State; size?: number; label?: string }) {
  return (
    <svg
      className="ui-mark"
      data-state={state}
      data-tone={STATE_TONE[state]}
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      role={label ? "img" : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    >
      <g key={state} className="ui-mark__shape">
      {state === "ok" && (
        <>
          <rect x="0.5" y="0.5" width="15" height="15" rx="3.5" fill="var(--tone-soft)" />
          <path d="M4.6 8.3l2.3 2.3 4.5-4.8" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        </>
      )}
      {state === "idle" && <rect x="1.8" y="1.8" width="12.4" height="12.4" rx="2.6" stroke="currentColor" strokeWidth="1.6" />}
      {state === "busy" && (
        <rect
          className="ui-mark__dash"
          x="1.8"
          y="1.8"
          width="12.4"
          height="12.4"
          rx="2.6"
          stroke="currentColor"
          strokeWidth="1.7"
          pathLength="40"
          strokeDasharray="3 2"
        />
      )}
      {state === "fail" && (
        <>
          <rect x="0.5" y="0.5" width="15" height="15" rx="3.5" fill="var(--tone-soft)" />
          <path d="M5.4 5.4l5.2 5.2M10.6 5.4l-5.2 5.2" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </>
      )}
      {state === "warn" && (
        <>
          <rect x="0.5" y="0.5" width="15" height="15" rx="3.5" fill="var(--tone-soft)" />
          <path d="M8 4.2v4.6M8 11.4v.4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </>
      )}
      </g>
    </svg>
  );
}

/** A rubber stamp: the one place the app is allowed to shout. It only lands when `animate` is set,
 *  which is for a moment that earns it, never for a screen that merely loads. */
export function Stamp({
  children,
  tone = "accent",
  size = "md",
  animate,
  className,
}: {
  children: ReactNode;
  tone?: "accent" | "ok" | "warn" | "danger" | "neutral";
  size?: "sm" | "md" | "lg";
  animate?: boolean;
  className?: string;
}) {
  return (
    <span className={cx("ui-stamp", className)} data-tone={tone} data-size={size} data-animate={animate || undefined}>
      <span>{children}</span>
    </span>
  );
}

export type Step = { id: string; label: string; description?: string };

/** Progress through an ordered flow as numbered squares. Steps up to `reachable` are clickable. */
export function Steps({
  steps,
  current,
  onSelect,
  reachable,
  orientation = "vertical",
  className,
}: {
  steps: Step[];
  current: string;
  onSelect?: (id: string) => void;
  /** Index of the furthest step the user may jump to. Defaults to the current one, so only earlier
   *  steps are clickable. */
  reachable?: number;
  orientation?: "vertical" | "horizontal";
  className?: string;
}) {
  const at = Math.max(0, steps.findIndex((s) => s.id === current));
  const reach = reachable ?? at;
  return (
    <ol className={cx("ui-steps", className)} data-orientation={orientation} aria-label="Progress">
      {steps.map((step, i) => {
        const place = i < at ? "done" : i === at ? "current" : "next";
        const body = (
          <>
            <span className="ui-steps__box" aria-hidden>
              {i + 1}
            </span>
            <span className="ui-steps__text">
              <span className="ui-steps__label">{step.label}</span>
              {step.description && <span className="ui-steps__desc">{step.description}</span>}
            </span>
          </>
        );
        return (
          <li key={step.id} className="ui-steps__item" data-place={place} aria-current={place === "current" ? "step" : undefined}>
            {onSelect && i <= reach && place !== "current" ? (
              <button type="button" className="ui-steps__btn" onClick={() => onSelect(step.id)}>
                {body}
              </button>
            ) : (
              <div className="ui-steps__btn">{body}</div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
