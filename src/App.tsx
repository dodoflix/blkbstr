import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import {
  AppShell,
  CommandPalette,
  Icon,
  Kbd,
  Titlebar,
  ConfirmProvider,
  useConfirm,
  NavItem,
  Sidebar,
  SidebarToggle,
  useLocalStorage,
  useMediaQuery,
  ToastProvider,
  TooltipProvider,
  ThemeProvider,
  useAppearance,
  useHotkey,
  type Command,
} from "./ui";
import * as api from "./api";
import { BackendProvider, useBackend } from "./features/backend";
import { Brand } from "./features/Brand";
import { WindowControls, WindowFrame } from "./features/WindowControls";
import { ConfigsPage } from "./features/ConfigsPage";
import { Dashboard, HomePlaceholder } from "./features/Dashboard";
import { Onboarding } from "./features/Onboarding";
import { LogsPage } from "./features/LogsPage";
import { PrefsProvider, usePrefs } from "./features/prefs";
import { discardPaused, pauseForSetup, resumeAfterSetup } from "./features/setup";
import { SetupWizard } from "./features/SetupWizard";
import { SystemPage } from "./features/SystemPage";

type Page = "home" | "configs" | "logs" | "system" | "developer";

/** Developer tools exist only in dev builds: the constant folds to null and the chunk is never emitted. */
const DeveloperPage = import.meta.env.DEV ? lazy(() => import("./features/dev/DeveloperPage")) : null;

/** Home is the setup wizard until a config is saved, then a status page. Leaving the wizard is
 *  explicit so finishing it can show its last step. */
function Home({ wizard, setWizard, onSetup }: { wizard: boolean | undefined; setWizard: (on: boolean) => void; onSetup: () => void }) {
  const { ready, configs, act } = useBackend();
  const { onboarded } = usePrefs();
  useEffect(() => {
    if (ready && wizard === undefined) setWizard(configs.length === 0);
  }, [ready, wizard, configs.length, setWizard]);

  if (!onboarded) return <Onboarding />;
  if (!ready || wizard === undefined) return <HomePlaceholder />;
  return (
    <>
      {wizard ? (
        <SetupWizard
          canCancel={configs.length > 0}
          onCancel={() => {
            setWizard(false);
            void act(resumeAfterSetup);
          }}
          onDone={() => {
            discardPaused();
            setWizard(false);
          }}
        />
      ) : (
        <Dashboard onSetup={onSetup} />
      )}
    </>
  );
}

const THEME_NEXT = { system: "light", light: "dark", dark: "system" } as const;
const THEME_LABEL = { system: "System", light: "Light", dark: "Dark" } as const;
const THEME_ICON = { system: "monitor", light: "sun", dark: "moon" } as const;

function Shell() {
  const [page, setPageState] = useState<Page>(() => {
    const hash = location.hash.slice(1);
    return (["home", "configs", "logs", "system", "developer"] as const).find((p) => p === hash && (p !== "developer" || import.meta.env.DEV)) ?? "home";
  });
  // Pages stay mounted once visited, so leaving one keeps its state.
  const [visited, setVisited] = useState<Page[]>([page]);
  const setPage = (next: Page) => {
    location.hash = next;
    setPageState(next);
    setVisited((v) => (v.includes(next) ? v : [...v, next]));
  };
  const [wizard, setWizard] = useState<boolean>();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const { theme, setTheme } = useAppearance();
  // Narrow windows collapse the sidebar on their own; once the user toggles it, their choice wins.
  const narrow = useMediaQuery("(max-width: 860px)");
  const [sidebarPref, setSidebarPref] = useLocalStorage<"collapsed" | "expanded" | null>("blkbstr.sidebar", null);
  const collapsed = sidebarPref ? sidebarPref === "collapsed" : narrow;
  const { advanced, setAdvanced, onboarded } = usePrefs();
  const { status, configs, act } = useBackend();

  useHotkey("mod+k", (e) => {
    e.preventDefault();
    setPaletteOpen((o) => !o);
  }, { allowInInput: true });

  const commands = useMemo<Command[]>(
    () => [
      { id: "home", label: "Go to Home", icon: "home", run: () => setPage("home") },
      { id: "configs", label: "Go to Configs", icon: "layers", run: () => setPage("configs") },
      { id: "logs", label: "Go to Logs", icon: "terminal", run: () => setPage("logs") },
      { id: "system", label: "Go to System", icon: "cpu", run: () => setPage("system") },
      ...(import.meta.env.DEV ? [{ id: "developer", label: "Go to Developer", icon: "bug" as const, run: () => setPage("developer") }] : []),
      { id: "setup", label: "Run setup", icon: "wand", keywords: "wizard find strategy", run: () => void openSetup() },
      status?.running
        ? { id: "off", label: "Turn off", icon: "power", run: () => void act(api.engineStop) }
        : { id: "on", label: "Turn on", icon: "power", hint: configs[0], run: () => configs[0] && void act(async () => api.engineStart(await api.loadConfig(configs[0]))) },
      { id: "details", label: advanced ? "Hide details" : "Show details", icon: "sliders", keywords: "advanced", run: () => setAdvanced(!advanced) },
      {
        id: "theme",
        label: `Theme: ${THEME_LABEL[theme]}`,
        hint: "Click to switch",
        icon: THEME_ICON[theme],
        keywords: "dark light system appearance",
        keepOpen: true,
        run: () => setTheme(THEME_NEXT[theme]),
      },
    ],
    [status?.running, configs, advanced, act, setAdvanced, theme, setTheme],
  );

  const confirm = useConfirm();
  /** Every way into setup comes through here, because setup measures with Blockbuster off. */
  const openSetup = async () => {
    if (status?.running) {
      const ok = await confirm({
        title: "Turn Blockbuster off to run setup?",
        description:
          "Setup has to test your connection without Blockbuster, or every site would look like it already loads and the search would find nothing to fix. Blocked sites stop loading while it runs. Leaving setup without saving turns your current config back on; closing the app instead leaves it off.",
        confirmLabel: "Turn off and continue",
        tone: "warn",
      });
      if (!ok || !(await act(() => pauseForSetup(status)))) return;
    }
    setPage("home");
    setWizard(true);
  };

  return (
    <>
      <a href="#main" className="ui-sr-only">Skip to content</a>
      <AppShell
        scrollKey={page}
        collapsed={collapsed}
        titlebar={
          <Titlebar
            left={<Brand onClick={() => { setPage("home"); }} />}
            right={
              <>
                {onboarded && (
                  <button type="button" className="titlebar-search" onClick={() => setPaletteOpen(true)} aria-label="Commands">
                    <Icon name="search" size={14} />
                    <span>Commands</span>
                    <Kbd>Ctrl K</Kbd>
                  </button>
                )}
                <WindowControls />
              </>
            }
          />
        }
        sidebar={
          <Sidebar
            footer={onboarded && <SidebarToggle collapsed={collapsed} onToggle={() => setSidebarPref(collapsed ? "expanded" : "collapsed")} />}
          >
            {onboarded && (<>
            <NavItem icon="home" label="Home" active={page === "home"} onClick={() => setPage("home")} />
            <NavItem icon="layers" label="Configs" active={page === "configs"} onClick={() => setPage("configs")} />
            <NavItem icon="terminal" label="Logs" active={page === "logs"} onClick={() => setPage("logs")} />
            <NavItem icon="cpu" label="System" active={page === "system"} onClick={() => setPage("system")} />
            {import.meta.env.DEV && <NavItem icon="bug" label="Developer" active={page === "developer"} onClick={() => setPage("developer")} />}
            </>)}
          </Sidebar>
        }
      >
        {visited.includes("home") && (
          <div className="ui-page" hidden={page !== "home"}>
            <Home wizard={wizard} setWizard={setWizard} onSetup={() => void openSetup()} />
          </div>
        )}
        {visited.includes("configs") && (
          <div className="ui-page" hidden={page !== "configs"}>
            <ConfigsPage onSetup={() => void openSetup()} />
          </div>
        )}
        {visited.includes("logs") && (
          <div className="ui-page" hidden={page !== "logs"}>
            <LogsPage active={page === "logs"} />
          </div>
        )}
        {visited.includes("system") && (
          <div className="ui-page" hidden={page !== "system"}>
            <SystemPage />
          </div>
        )}
        {DeveloperPage && page === "developer" && (
          <Suspense fallback={null}>
            <div className="ui-page">
              <DeveloperPage />
            </div>
          </Suspense>
        )}
      </AppShell>
      <WindowFrame />
      <CommandPalette commands={commands} open={paletteOpen} onOpenChange={setPaletteOpen} />
    </>
  );
}

export default function App() {
  return (
    <ThemeProvider>
    <ToastProvider>
      <TooltipProvider>
        <ConfirmProvider>
          <PrefsProvider>
            <BackendProvider>
              <Shell />
            </BackendProvider>
          </PrefsProvider>
        </ConfirmProvider>
      </TooltipProvider>
    </ToastProvider>
    </ThemeProvider>
  );
}
