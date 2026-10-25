import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "../ui";

export const inTauri = "__TAURI_INTERNALS__" in window;
/** The developer page embeds the app in iframes; those must not draw their own window controls. */
const embedded = window.self !== window.top;

type Edge = Parameters<ReturnType<typeof getCurrentWindow>["startResizeDragging"]>[0];
const EDGES: Edge[] = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"];

/** Maximised and fullscreen state of the Tauri window; both false elsewhere. */
function useWindowState() {
  const [state, setState] = useState({ maximized: false, fullscreen: false });

  useEffect(() => {
    if (!inTauri || embedded) return;
    const win = getCurrentWindow();
    const sync = () =>
      void Promise.all([win.isMaximized(), win.isFullscreen()]).then(([maximized, fullscreen]) => setState({ maximized, fullscreen }));
    sync();
    const off = win.onResized(sync);
    return () => void off.then((fn) => fn());
  }, []);

  return state;
}

/** A 1px outline around the whole window, drawn only when it is a floating window. An undecorated
 *  window has no frame of its own, so without this its edges and corners disappear into the desktop. */
export function WindowFrame() {
  const { maximized, fullscreen } = useWindowState();
  if (!inTauri || embedded || maximized || fullscreen) return null;
  return <div className="ui-frame" aria-hidden />;
}

/** Minimise, maximise and close for the undecorated window; nothing outside Tauri. */
export function WindowControls() {
  const { maximized } = useWindowState();
  if (!inTauri || embedded) return null;
  const win = getCurrentWindow();
  return (
    <>
      <div className="ui-wincontrols">
        <button type="button" aria-label="Minimize" onClick={() => void win.minimize()}>
          <Icon name="minus" size={16} />
        </button>
        <button type="button" aria-label={maximized ? "Restore" : "Maximize"} onClick={() => void win.toggleMaximize()}>
          <Icon name={maximized ? "restore" : "maximize"} size={14} />
        </button>
        <button type="button" data-close aria-label="Close" onClick={() => void win.close()}>
          <Icon name="x" size={16} />
        </button>
      </div>
      {/* An undecorated window has no frame to grab, so the edges are drawn as hit areas. */}
      {!maximized &&
        EDGES.map((edge) => (
          <div key={edge} className="ui-resize" data-edge={edge} onMouseDown={() => void win.startResizeDragging(edge)} />
        ))}
    </>
  );
}
