import { useState } from "react";
import {
  Badge,
  Button,
  CodeBlock,
  Drawer,
  DropdownMenu,
  EmptyState,
  FileDrop,
  Columns,
  Panel,
  useMediaQuery,
  IconButton,
  KeyValue,
  Dialog,
  Notice,
  PageHeader,
  RowList,
  Stack,
  Tabs,
  Text,
  useAsync,
  useConfirm,
  useToast,
  Icon,
} from "../ui";
import * as api from "../api";
import { useBackend } from "./backend";
import { plural } from "./format";
import { usePrefs } from "./prefs";

const download = (name: string, text: string) => {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = Object.assign(document.createElement("a"), { href: url, download: `${name}.json` });
  a.click();
  URL.revokeObjectURL(url);
};

function ConfigDetail({ name }: { name: string }) {
  const { advanced } = usePrefs();
  const detail = useAsync(async () => {
    const config = await api.loadConfig(name);
    const [preview, warnings] = await Promise.all([api.previewConfig(config), api.lintConfig(config)]);
    return { config, preview, warnings };
  }, [name]);

  if (!detail.data) return <Text color={3}>{detail.error ?? "Loading..."}</Text>;
  const { config, preview, warnings } = detail.data;
  return (
    <Stack gap={4}>
      {config.notes && <Text as="p" color={2}>{config.notes}</Text>}
      {warnings.map((w, i) => (
        <Notice key={i} tone="warn" title={w.strategy ?? "Config"}>
          {w.message}
        </Notice>
      ))}
      <Tabs
        items={[
          {
            value: "overview",
            label: "Overview",
            content: (
              <KeyValue
                items={config.strategies.map((s) => ({
                  label: s.name,
                  value: (
                    <Stack gap={1}>
                      <Text>{s.enabled ? "On" : "Off"}{s.filter.tcp ? `, TCP ${s.filter.tcp}` : ""}{s.filter.udp ? `, UDP ${s.filter.udp}` : ""}</Text>
                      <Text size="sm" color={2} mono>{s.actions.map((a) => a.function).join(" > ")}</Text>
                    </Stack>
                  ),
                }))}
              />
            ),
          },
          { value: "params", label: "Parameter file", content: <CodeBlock title="What the engine is given" code={preview} /> },
          ...(advanced ? [{ value: "json", label: "JSON", content: <CodeBlock title={`${name}.json`} code={JSON.stringify(config, null, 2)} maxHeight={420} /> }] : []),
        ]}
      />
    </Stack>
  );
}

export function ConfigsPage({ onSetup }: { onSetup: () => void }) {
  const { configs, status, act, refreshConfigs } = useBackend();
  const toast = useToast();
  const confirm = useConfirm();
  const [open, setOpen] = useState<string>();
  // With room for two columns the detail sits beside the list; otherwise it opens as a drawer.
  const wide = useMediaQuery("(min-width: 1200px)");
  const shown = open && configs.includes(open) ? open : wide ? configs[0] : undefined;
  const [importing, setImporting] = useState(false);
  const active = status?.running ? status.active_config : undefined;

  const use = (name: string) => act(async () => api.engineStart(await api.loadConfig(name)), `${name} is running`);
  const remove = async (name: string) => {
    const ok = await confirm({ title: `Delete ${name}?`, description: "The file is removed from your configs folder. This cannot be undone.", confirmLabel: "Delete", tone: "danger" });
    if (ok) await act(() => api.deleteConfig(name), `Deleted ${name}`);
  };
  const exportOne = async (name: string) => download(name, JSON.stringify(await api.loadConfig(name), null, 2));
  const importFile = async ([file]: File[]) => {
    try {
      const config = JSON.parse(await file.text()) as api.Config;
      await api.saveConfig(config);
      await refreshConfigs();
      setImporting(false);
      toast({ title: `Imported ${config.name}`, tone: "ok" });
    } catch (e) {
      toast({ title: "That file could not be imported", description: String(e), tone: "danger", durationMs: 9000 });
    }
  };

  return (
    <div>
      <PageHeader
        title="Configs"
        description="Saved ways of unblocking. Each is a portable file you can share."
        actions={
          <>
            <Button variant="outline" tone="neutral" icon="upload" onClick={() => setImporting(true)}>
              Import
            </Button>
            <Button icon="wand" onClick={onSetup}>
              Find a new one
            </Button>
          </>
        }
      />
      <Columns min="30rem" gap={5}>
        <Panel
          title="Saved"
          description={configs.length > 0 ? `${plural(configs.length, "config")}` : undefined}
        >
          <RowList
            label="Saved configs"
            empty={
              <EmptyState icon="layers" title="No saved configs yet" action={<Button icon="wand" onClick={onSetup}>Run setup</Button>}>
                Setup finds a strategy that works on your network and saves it here.
              </EmptyState>
            }
            items={configs.map((name) => ({
              id: name,
              leading: <Icon name="layers" size={18} />,
              title: (
                <span style={{ display: "inline-flex", gap: "0.5rem", alignItems: "center" }}>
                  {name}
                  {name === active && <Badge tone="ok">running</Badge>}
                </span>
              ),
              onClick: () => setOpen(name),
              trailing: (
                <>
                  {name === active ? (
                    <Button size="sm" variant="soft" tone="neutral" icon="stop" onClick={(e) => { e.stopPropagation(); void act(api.engineStop); }}>
                      Stop
                    </Button>
                  ) : (
                    <Button size="sm" variant="soft" icon="play" onClick={(e) => { e.stopPropagation(); void use(name); }}>
                      Use
                    </Button>
                  )}
                  <span onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu
                      trigger={<IconButton icon="more" label={`More for ${name}`} size="sm" />}
                      entries={[
                        { label: "Open", icon: "eye", onSelect: () => setOpen(name) },
                        { label: "Export", icon: "download", onSelect: () => void exportOne(name) },
                        { type: "separator" },
                        { label: "Delete", icon: "trash", tone: "danger", onSelect: () => void remove(name) },
                      ]}
                    />
                  </span>
                </>
              ),
            }))}
          />
        </Panel>
        {wide && shown && (
          <Panel title={shown} description="Saved config" className="configs__detail">
            <ConfigDetail key={shown} name={shown} />
          </Panel>
        )}
      </Columns>

      {!wide && (
        <Drawer open={Boolean(open)} onOpenChange={(o) => !o && setOpen(undefined)} title={open ?? ""} description="Saved config">
          {open && <ConfigDetail name={open} />}
        </Drawer>
      )}

      <Dialog open={importing} onOpenChange={setImporting} title="Import a config" description="Choose a JSON config someone shared with you. Nothing runs until you use it.">
        <FileDrop accept="application/json,.json" onFiles={(f) => void importFile(f)} title="Drop a config file here" />
      </Dialog>
    </div>
  );
}
