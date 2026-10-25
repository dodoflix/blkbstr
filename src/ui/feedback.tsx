import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { Progress as RProgress, Toast as RToast } from "radix-ui";
import { cx } from "./cx";
import { Icon, type IconName } from "./Icon";
import { StateMark } from "./marks";
import { Button, IconButton, Text, Heading } from "./primitives";
import { STATE_TONE, type State, type Tone } from "./types";

const TONE_STATE: Record<Tone, State> = { neutral: "idle", accent: "busy", ok: "ok", warn: "warn", danger: "fail" };

/** Inline message. Always carries a mark, so tone never rides on colour alone. */
export function Notice({
  tone = "neutral",
  title,
  children,
  action,
  onDismiss,
  className,
}: {
  tone?: Tone;
  title?: ReactNode;
  children?: ReactNode;
  action?: ReactNode;
  onDismiss?: () => void;
  className?: string;
}) {
  return (
    <div className={cx("ui-notice", className)} data-tone={tone} role={tone === "danger" || tone === "warn" ? "alert" : "status"}>
      <span className="ui-notice__mark">
        {tone === "neutral" ? <Icon name="info" size={18} /> : <StateMark state={TONE_STATE[tone]} size={18} />}
      </span>
      <div className="ui-notice__body">
        {title && <strong className="ui-notice__title">{title}</strong>}
        {children && <div className="ui-notice__text">{children}</div>}
      </div>
      {action && <div className="ui-notice__action">{action}</div>}
      {onDismiss && <IconButton icon="x" label="Dismiss" size="sm" onClick={onDismiss} />}
    </div>
  );
}

export function Progress({
  value,
  max = 100,
  tone = "accent",
  label,
  indeterminate,
}: {
  value?: number;
  max?: number;
  tone?: Tone;
  label: string;
  indeterminate?: boolean;
}) {
  const pct = indeterminate || value === undefined ? undefined : Math.min(100, (value / max) * 100);
  return (
    <RProgress.Root className="ui-progress" data-tone={tone} value={indeterminate ? null : (value ?? null)} max={max} aria-label={label}>
      <RProgress.Indicator
        className="ui-progress__bar"
        data-indeterminate={pct === undefined || undefined}
        style={pct === undefined ? undefined : { transform: `scaleX(${pct / 100})` }}
      />
    </RProgress.Root>
  );
}

/** A value within a known range (latency, score). Not progress: it does not move toward done. */
export function Meter({
  value,
  min = 0,
  max = 100,
  tone = "accent",
  label,
}: {
  value: number;
  min?: number;
  max?: number;
  tone?: Tone;
  label: string;
}) {
  const pct = Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100));
  return (
    <div className="ui-meter" role="meter" aria-label={label} aria-valuenow={value} aria-valuemin={min} aria-valuemax={max} data-tone={tone}>
      <div className="ui-meter__fill" style={{ width: `${pct}%` }} />
    </div>
  );
}

export function EmptyState({
  icon = "file",
  title,
  children,
  action,
}: {
  icon?: IconName;
  title: ReactNode;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="ui-empty">
      <span className="ui-empty__icon">
        <Icon name={icon} size={22} />
      </span>
      <Heading level={3} size="lg">
        {title}
      </Heading>
      {children && (
        <Text as="p" color={2} className="ui-empty__text">
          {children}
        </Text>
      )}
      {action}
    </div>
  );
}

export function ErrorState({ title = "Something went wrong", error, onRetry }: { title?: ReactNode; error?: string; onRetry?: () => void }) {
  return (
    <EmptyState
      icon="alert"
      title={title}
      action={
        onRetry && (
          <Button variant="soft" tone="neutral" icon="refresh" onClick={onRetry}>
            Try again
          </Button>
        )
      }
    >
      {error}
    </EmptyState>
  );
}

/* Toasts */
export type ToastInput = { title: ReactNode; description?: ReactNode; tone?: Tone; durationMs?: number; action?: { label: string; onClick: () => void } };
type ToastItem = ToastInput & { id: number };
const ToastContext = createContext<((toast: ToastInput) => void) | null>(null);

export function useToast() {
  const push = useContext(ToastContext);
  if (!push) throw new Error("useToast needs a ToastProvider");
  return push;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const push = useCallback((toast: ToastInput) => setItems((prev) => [...prev, { ...toast, id: Date.now() + Math.random() }]), []);
  const value = useMemo(() => push, [push]);
  return (
    <ToastContext.Provider value={value}>
      <RToast.Provider swipeDirection="right" duration={5000}>
        {children}
        {items.map((t) => (
          <RToast.Root
            key={t.id}
            className="ui-toast"
            data-tone={t.tone ?? "neutral"}
            duration={t.durationMs}
            onOpenChange={(open) => !open && setItems((prev) => prev.filter((x) => x.id !== t.id))}
          >
            <StateMark state={TONE_STATE[t.tone ?? "neutral"]} size={18} />
            <div className="ui-toast__body">
              <RToast.Title className="ui-toast__title">{t.title}</RToast.Title>
              {t.description && <RToast.Description className="ui-toast__desc">{t.description}</RToast.Description>}
            </div>
            {t.action && (
              <RToast.Action altText={t.action.label} asChild>
                <Button size="sm" variant="soft" tone={STATE_TONE[TONE_STATE[t.tone ?? "neutral"]]} onClick={t.action.onClick}>
                  {t.action.label}
                </Button>
              </RToast.Action>
            )}
            <RToast.Close asChild>
              <IconButton icon="x" label="Dismiss" size="sm" />
            </RToast.Close>
          </RToast.Root>
        ))}
        <RToast.Viewport className="ui-toasts" label="Notifications" />
      </RToast.Provider>
    </ToastContext.Provider>
  );
}
