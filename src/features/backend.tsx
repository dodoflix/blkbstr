import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as api from "../api";
import { useAsync, usePoll, useToast } from "../ui";
import { selectedHosts } from "./sites";

export type Backend = {
  daemon?: api.DaemonInfo;
  status?: api.EngineStatus;
  service?: api.ServiceStatus;
  env?: api.Environment;
  configs: string[];
  /** True once daemon, engine status and configs have all answered, so screens never flash a guess. */
  ready: boolean;
  refresh: () => Promise<void>;
  refreshEnv: () => Promise<void>;
  refreshConfigs: () => Promise<void>;
  /** The last reachability result, shared so it survives moving between screens. */
  report?: api.Report;
  checking: boolean;
  check: () => Promise<void>;
  /** Runs a backend action, reports failure as a toast, and refreshes either way. */
  act: (fn: () => Promise<unknown>, success?: string) => Promise<boolean>;
};

const UPDATE_CHECK_MS = 6 * 60 * 60 * 1000;

export const BackendContext = createContext<Backend | null>(null);
const Ctx = BackendContext;

export const useBackend = () => {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useBackend needs a BackendProvider");
  return ctx;
};

/** Owns daemon, engine, service and environment state so screens do not each poll the daemon. */
export function BackendProvider({ children }: { children: ReactNode }) {
  const toast = useToast();
  const [daemon, setDaemon] = useState<api.DaemonInfo>();
  const [status, setStatus] = useState<api.EngineStatus>();
  const [service, setService] = useState<api.ServiceStatus>();

  const [report, setReport] = useState<api.Report>();
  const [checking, setChecking] = useState(false);

  const env = useAsync(api.detectEnvironment, []);
  const configs = useAsync(api.listConfigs, []);
  const reloadConfigs = configs.reload;

  const refresh = useCallback(async () => {
    const info = await api.daemonInfo();
    setDaemon(info);
    api.serviceStatus().then(setService, () => setService(undefined));
    if (!info.reachable) {
      setStatus(undefined);
      return;
    }
    try {
      setStatus(await api.engineStatus());
    } catch {
      setStatus(undefined);
    }
  }, []);

  const check = useCallback(async () => {
    setChecking(true);
    try {
      setReport(await api.checkReachability(await selectedHosts()));
    } catch (e) {
      toast({ title: "The site check failed", description: String(e), tone: "danger" });
    } finally {
      setChecking(false);
    }
  }, [toast]);

  // Not usePoll: that also fires whenever the window is shown, which would hit GitHub on every tray click.
  const announced = useRef<string | undefined>(undefined);
  useEffect(() => {
    const look = async () => {
      try {
        const update = await api.checkAppUpdate();
        if (!update || update.version === announced.current) return;
        announced.current = update.version;
        toast({ title: `Blockbuster ${update.version} is available`, description: "Open System to install it.", tone: "warn", durationMs: 15000 });
      } catch {
        // Offline or GitHub unreachable: the next round tries again.
      }
    };
    void look();
    const timer = setInterval(() => void look(), UPDATE_CHECK_MS);
    return () => clearInterval(timer);
  }, [toast]);

  // The daemon does not push, and uptime is a lie the moment it stops being re-read.
  usePoll(refresh, 5000);

  const act = useCallback(
    async (fn: () => Promise<unknown>, success?: string) => {
      try {
        await fn();
        if (success) toast({ title: success, tone: "ok" });
        return true;
      } catch (e) {
        toast({ title: "That did not work", description: String(e), tone: "danger", durationMs: 9000 });
        return false;
      } finally {
        await refresh();
        void reloadConfigs();
      }
    },
    [refresh, toast, reloadConfigs],
  );

  const value = useMemo<Backend>(
    () => ({
      daemon,
      status,
      service,
      env: env.data,
      configs: configs.data ?? [],
      ready: daemon !== undefined && configs.data !== undefined && (!daemon.reachable || status !== undefined),
      report,
      checking,
      check,
      refresh,
      refreshEnv: env.reload,
      refreshConfigs: configs.reload,
      act,
    }),
    [daemon, status, service, env.data, env.reload, configs.data, configs.reload, report, checking, check, refresh, act],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
