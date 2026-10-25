import { useState } from "react";
import {
  Button,
  Panel,
  RowList,
  Stack,
  Tabs,
  Text,
  TextField,
  Badge,
  useConfirm,
  useLocalStorage,
  useNow,
  usePoll,
  useToast,
} from "../ui";
import * as api from "../api";
import { useBackend } from "./backend";
import { plural, until, uptime } from "./format";
import { HostModePicker, modeOf, usesList, withMode, type HostMode } from "./hostmode";
import { normalizeSite, validSite } from "./sites";

/** The config the panels act on: the running one, else the one Home would start. */
export function useTargetConfig() {
  const { status, configs } = useBackend();
  const [last] = useLocalStorage<string | undefined>("blkbstr.lastConfig", undefined);
  const name = status?.active_config ?? (last && configs.includes(last) ? last : configs[0]);
  const [config, setConfig] = useState<api.Config>();
  return { name, config, setConfig };
}

/** Home: which sites get the strategy. */
type Target = ReturnType<typeof useTargetConfig>;

export function HostModePanel({ target }: { target: Target }) {
  const { status, act } = useBackend();
  const { name, config, setConfig } = target;
  const trial = status?.revert_in_seconds !== undefined;
  if (!name) return null;

  const change = (mode: HostMode) =>
    act(async () => {
      const updated = withMode(await api.loadConfig(name), mode);
      await api.saveConfig(updated);
      // A running config only changes by restarting it, which is a brief gap in the workaround.
      if (status?.running && status.active_config === name) await api.engineStart(updated);
      setConfig(updated);
    });

  return (
    <Panel title="Which sites" description="What the strategy is applied to." className="mode-panel">
      <HostModePicker value={config ? modeOf(config) : "auto"} disabled={!config || trial} onChange={(mode) => void change(mode)} />
    </Panel>
  );
}

const SPLIT = /[\s,;]+/;

function summary(list: api.HostsStatus, now: number): string {
  if (list.checking) return "Checking which of these load without help.";
  if (!list.last_check) return list.next_check ? `First check in ${until(list.next_check, now)}.` : "Sites are added as they fail to load.";
  const dropped = list.last_dropped.length;
  const result = dropped === 0 ? "All still blocked." : `${plural(dropped, "site")} loaded again and ${dropped === 1 ? "was" : "were"} dropped.`;
  const ago = Math.floor(now / 1000) - list.last_check < 60 ? "just now" : `${uptime(list.last_check, now)} ago`;
  const next = list.next_check ? ` Next check in ${until(list.next_check, now)}.` : "";
  return `Checked ${ago}. ${result}${next}`;
}

/** One list: add, edit, remove. The learned list can also be re-checked. */
function HostListEditor({ list, status, used, onChanged }: { list: api.HostList; status?: api.HostsStatus; used: boolean; onChanged: () => Promise<void> }) {
  const { act } = useBackend();
  const toast = useToast();
  const confirm = useConfirm();
  const now = useNow(30_000);
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState<string>();
  const [edit, setEdit] = useState("");
  const hosts = status?.hosts ?? [];

  const run = (fn: () => Promise<unknown>, success?: string) => act(fn, success).then(onChanged);

  const add = async () => {
    const entered = draft.split(SPLIT).map(normalizeSite).filter(Boolean);
    const good = entered.filter(validSite);
    const bad = entered.filter((h) => !validSite(h));
    if (bad.length > 0) toast({ title: `Not added: ${bad.join(", ")}`, description: "Use site names like example.com.", tone: "warn" });
    if (good.length === 0) return;
    setDraft("");
    await run(() => api.hostsAdd(list, good));
  };

  const save = async (old: string) => {
    const next = normalizeSite(edit);
    setEditing(undefined);
    if (next === old) return;
    if (!validSite(next)) {
      toast({ title: `${edit.trim() || "That"} is not a site name`, description: "Use a name like example.com.", tone: "warn" });
      return;
    }
    await run(async () => {
      await api.hostsAdd(list, [next]);
      await api.hostsRemove(list, [old]);
    });
  };

  const clear = async () => {
    if (!(await confirm({ title: `Remove all ${hosts.length} sites?`, description: list === "auto" ? "The engine learns them again as they fail to load." : "Your list will be empty.", confirmLabel: "Remove all", tone: "danger" }))) return;
    await run(() => api.hostsRemove(list, hosts));
  };

  const items = hosts.map((host) => ({
    id: host,
    title:
      editing === host ? (
        <TextField
          aria-label={`Edit ${host}`}
          autoFocus
          value={edit}
          onChange={(e) => setEdit(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void save(host);
            if (e.key === "Escape") setEditing(undefined);
          }}
        />
      ) : (
        host
      ),
    trailing:
      editing === host ? (
        <>
          <Button size="sm" onClick={() => void save(host)}>
            Save
          </Button>
          <Button size="sm" variant="soft" tone="neutral" onClick={() => setEditing(undefined)}>
            Cancel
          </Button>
        </>
      ) : (
        <>
          <Button size="sm" variant="ghost" tone="neutral" icon="edit" aria-label={`Edit ${host}`} onClick={() => { setEditing(host); setEdit(host); }} />
          <Button size="sm" variant="ghost" tone="neutral" icon="trash" aria-label={`Remove ${host}`} onClick={() => void run(() => api.hostsRemove(list, [host]))} />
        </>
      ),
  }));

  return (
    <Stack gap={4}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <Stack gap={2}>
          <div className="host-add">
            <TextField aria-label="Add sites" placeholder="example.com, another.org" value={draft} onChange={(e) => setDraft(e.currentTarget.value)} />
            <Button type="submit" icon="plus" disabled={!draft.trim()}>
              Add
            </Button>
          </div>
          <Text size="sm" color={3}>
            {list === "auto"
              ? "Sites already known to be blocked, so they work on the first visit. A site covers its subdomains."
              : "A site covers its subdomains: example.com also covers www.example.com."}
          </Text>
        </Stack>
      </form>
      {!used && status && (
        <Text size="sm" color={3}>
          The current setting does not use this list.
        </Text>
      )}
      {list === "auto" && status && hosts.length > 0 && (
        <Text size="sm" color={2}>
          {status.can_recheck ? summary(status, now) : "Sites stay on this list until you remove them: re-checking them without interrupting your connection is not available on this system."}
        </Text>
      )}
      <RowList
        label={list === "auto" ? "Learned sites" : "My sites"}
        items={items}
        empty={<Text color={3}>{list === "auto" ? "Nothing yet. Sites show up here after they fail to load." : "Nothing yet. Add the sites you want unblocked."}</Text>}
      />
      {hosts.length > 0 && (
        <div className="host-actions">
          {list === "auto" && used && status?.can_recheck && (
            <Button size="sm" variant="soft" tone="neutral" icon="refresh" loading={status?.checking} onClick={() => void run(api.hostsCheck)}>
              Check now
            </Button>
          )}
          <Button size="sm" variant="ghost" tone="danger" icon="trash" onClick={() => void clear()}>
            Remove all
          </Button>
        </div>
      )}
    </Stack>
  );
}

/** Home: both lists, in tabs, at a fixed height so editing never moves the page. */
export function HostListsPanel({ target }: { target: Target }) {
  const { config, setConfig, name } = target;
  const [lists, setLists] = useState<Partial<Record<api.HostList, api.HostsStatus>>>({});
  const [tab, setTab] = useState<api.HostList>("auto");

  const load = async () => {
    const [loaded, auto, manual] = await Promise.all([
      name ? api.loadConfig(name).catch(() => undefined) : undefined,
      api.hosts("auto").catch(() => undefined),
      api.hosts("manual").catch(() => undefined),
    ]);
    setConfig(loaded);
    setLists({ auto, manual });
  };
  usePoll(load, 5000);

  const mode = config ? modeOf(config) : "auto";
  const count = (list: api.HostList) => lists[list]?.hosts.length ?? 0;

  return (
    <Panel title="Site lists" description="Add, edit or remove the sites the strategy applies to." className="lists-panel">
      <Tabs
        value={tab}
        onValueChange={(v) => setTab(v as api.HostList)}
        items={(["auto", "manual"] as const).map((list) => ({
          value: list,
          label: list === "auto" ? "Learned" : "My list",
          badge: <Badge>{count(list)}</Badge>,
          content: <HostListEditor list={list} status={lists[list]} used={usesList(mode, list)} onChanged={load} />,
        }))}
      />
    </Panel>
  );
}
