import {
  useCallback,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";

/** localStorage that survives being unavailable (private windows, blocked storage). */
export function useLocalStorage<T>(key: string, initial: T): [T, (next: T) => void] {
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(key);
      return raw === null ? initial : (JSON.parse(raw) as T);
    } catch {
      return initial;
    }
  });
  const set = useCallback(
    (next: T) => {
      setValue(next);
      try {
        localStorage.setItem(key, JSON.stringify(next));
      } catch {
        /* the value still holds for this session */
      }
    },
    [key],
  );
  return [value, set];
}

export type AsyncState<T> = {
  data?: T;
  error?: string;
  loading: boolean;
  reload: () => Promise<void>;
};

/** Runs `fn` on mount and whenever `deps` change. A stale response never overwrites a newer one. */
export function useAsync<T>(fn: () => Promise<T>, deps: unknown[] = []): AsyncState<T> {
  const [data, setData] = useState<T>();
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(true);
  const ticket = useRef(0);
  const latest = useRef(fn);
  latest.current = fn;

  const reload = useCallback(async () => {
    const mine = ++ticket.current;
    setLoading(true);
    try {
      const result = await latest.current();
      if (mine === ticket.current) {
        setData(result);
        setError(undefined);
      }
    } catch (e) {
      if (mine === ticket.current) setError(String(e));
    } finally {
      if (mine === ticket.current) setLoading(false);
    }
  }, []);

  useEffect(() => void reload(), deps);
  return { data, error, loading, reload };
}

/** Calls `fn` now and every `ms` while the window is visible. `ms: null` pauses. */
export function usePoll(fn: () => void | Promise<void>, ms: number | null) {
  const latest = useRef(fn);
  latest.current = fn;
  useEffect(() => {
    if (ms === null) return;
    const tick = () => {
      if (!document.hidden) void latest.current();
    };
    tick();
    const timer = setInterval(tick, ms);
    document.addEventListener("visibilitychange", tick);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", tick);
    };
  }, [ms]);
}

export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      const mq = window.matchMedia(query);
      mq.addEventListener("change", onChange);
      return () => mq.removeEventListener("change", onChange);
    },
    [query],
  );
  return useSyncExternalStore(subscribe, () => window.matchMedia(query).matches);
}

export function useDebounce<T>(value: T, ms = 200): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

export function useDisclosure(initial = false) {
  const [open, setOpen] = useState(initial);
  return {
    open,
    setOpen,
    show: useCallback(() => setOpen(true), []),
    hide: useCallback(() => setOpen(false), []),
    toggle: useCallback(() => setOpen((o) => !o), []),
  };
}

export function useClipboard(resetMs = 1600) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const copy = useCallback(
    async (text: string) => {
      try {
        await navigator.clipboard.writeText(text);
        setCopied(true);
        clearTimeout(timer.current);
        timer.current = setTimeout(() => setCopied(false), resetMs);
      } catch {
        setCopied(false);
      }
    },
    [resetMs],
  );
  useEffect(() => () => clearTimeout(timer.current), []);
  return { copied, copy };
}

/** "mod+k" matches Ctrl or Cmd. Ignored while typing in a field unless `allowInInput`. */
export function useHotkey(
  combo: string,
  handler: (event: KeyboardEvent) => void,
  { allowInInput = false, enabled = true } = {},
) {
  const latest = useRef(handler);
  latest.current = handler;
  useEffect(() => {
    if (!enabled) return;
    const parts = combo.toLowerCase().split("+");
    const key = parts[parts.length - 1];
    const wants = (m: string) => parts.includes(m);
    const onKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== key) return;
      if (wants("mod") !== (e.ctrlKey || e.metaKey)) return;
      if (wants("shift") !== e.shiftKey || wants("alt") !== e.altKey) return;
      const t = e.target as HTMLElement | null;
      const typing = t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));
      if (typing && !allowInInput) return;
      latest.current(e);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [combo, allowInInput, enabled]);
}

/** Re-renders every `ms`, for values derived from the clock (uptime, countdowns). */
export function useNow(ms = 1000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(timer);
  }, [ms]);
  return now;
}
