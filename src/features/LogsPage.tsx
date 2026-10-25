import { startTransition, useCallback, useDeferredValue, useEffect, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  Button,
  EmptyState,
  LogViewer,
  Notice,
  PageHeader,
  SearchField,
  Select,
  Sheet,
  Spacer,
  Stack,
  Switch,
  Text,
  Toolbar,
  useAsync,
  usePoll,
} from "../ui";
import * as api from "../api";

export function LogsPage({ active }: { active: boolean }) {
  const files = useAsync(api.listLogs, []);
  const [selected, setSelected] = useState<string>();
  const [text, setText] = useState("");
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const [exported, setExported] = useState<string>();
  const [error, setError] = useState<string>();

  const list = files.data ?? [];
  useEffect(() => setSelected((cur) => cur ?? list[0]?.name), [list]);

  const load = useCallback(async () => {
    if (!selected) return;
    try {
      const next = await api.readLog(selected);
      startTransition(() => setText(next));
    } catch (e) {
      setError(String(e));
    }
  }, [selected]);
  usePoll(load, active && follow ? 2000 : null);
  useEffect(() => void load(), [load]);

  // Typing in the filter must stay responsive while a large log re-filters.
  const deferred = useDeferredValue(filter);
  const needle = deferred.toLowerCase();
  const shown = text.split("\n").filter((l) => l && (!needle || l.toLowerCase().includes(needle))).length;

  const doExport = async () => {
    try {
      setExported(await api.exportDiagnostics(list.map((f) => f.name)));
    } catch (e) {
      setError(String(e));
    }
  };

  if (!files.loading && list.length === 0 && !files.error) {
    return (
      <div>
        <PageHeader title="Logs" />
        <Sheet>
          <EmptyState icon="terminal" title="No logs yet">
            The service writes logs once it has run.
          </EmptyState>
        </Sheet>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Logs"
        description="What the engine and the service have been doing."
        actions={
          <Button variant="outline" tone="neutral" icon="download" onClick={() => void doExport()}>
            Export for a bug report
          </Button>
        }
      />
      <Stack gap={3}>
        {(error || files.error) && <Notice tone="danger" onDismiss={() => setError(undefined)}>{error ?? files.error}</Notice>}
        {exported && (
          <Notice
            tone="ok"
            title="Exported"
            action={
              <Button size="sm" variant="soft" tone="neutral" icon="folder" onClick={() => void revealItemInDir(exported).catch(() => {})}>
                Show file
              </Button>
            }
            onDismiss={() => setExported(undefined)}
          >
            Written to <code>{exported}</code>. Read it before sending it anywhere: engine logs name the sites they saw.
          </Notice>
        )}
        <Toolbar>
          <Select
            aria-label="Log file"
            value={selected}
            onValueChange={setSelected}
            options={list.map((f) => ({ value: f.name, label: f.name }))}
          />
          <SearchField
            aria-label="Filter lines"
            placeholder="Filter lines"
            value={filter}
            onChange={(e) => setFilter(e.currentTarget.value)}
            onClear={() => setFilter("")}
            className="logs__filter"
          />
          <Spacer />
          <Switch size="sm" label="Follow" checked={follow} onCheckedChange={setFollow} />
        </Toolbar>
        <LogViewer text={text} filter={deferred} follow={follow} onFollowChange={setFollow} height="max(16rem, calc(100vh - 17.5rem))" />
        <Text size="sm" color={3}>
          {shown} line{shown === 1 ? "" : "s"}
          {needle && ` matching "${filter}"`}
        </Text>
      </Stack>
    </div>
  );
}
