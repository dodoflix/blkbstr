import { useState } from "react";
import {
  Accordion,
  Badge,
  Breadcrumbs,
  Button,
  ButtonGroup,
  Checkbox,
  Chip,
  CodeBlock,
  CommandPalette,
  CopyButton,
  Dialog,
  Disclosure,
  Divider,
  Drawer,
  DropdownMenu,
  EmptyState,
  ErrorState,
  Field,
  FileDrop,
  Grid,
  Heading,
  IconButton,
  Icon,
  iconNames,
  Kbd,
  KeyValue,
  LogViewer,
  Meter,
  NumberField,
  Notice,
  Pagination,
  Popover,
  Progress,
  RadioGroup,
  Row,
  RowList,
  ScrollArea,
  SearchField,
  Section,
  SegmentedControl,
  Select,
  Sheet,
  Skeleton,
  Slider,
  Spinner,
  Stack,
  Stamp,
  StateMark,
  Switch,
  Table,
  Tabs,
  TagInput,
  Text,
  TextArea,
  TextField,
  Steps,
  Timeline,
  Toolbar,
  Tooltip,
  useConfirm,
  useToast,
  type State,
  type Tone,
} from "./index";

const TONES: Tone[] = ["neutral", "accent", "ok", "warn", "danger"];
const STATES: State[] = ["ok", "idle", "busy", "fail", "warn"];

function Block({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Section title={title}>
      <Sheet>{children}</Sheet>
    </Section>
  );
}

/** Every component in src/ui, live. Rendered by the Developer page, which exists only in dev builds. */
export function Gallery() {
  const toast = useToast();
  const confirm = useConfirm();
  const [seg, setSeg] = useState("a");
  const [on, setOn] = useState(true);
  const [check, setCheck] = useState(true);
  const [radio, setRadio] = useState("two");
  const [sel, setSel] = useState("x");
  const [num, setNum] = useState(3);
  const [slide, setSlide] = useState(40);
  const [tags, setTags] = useState(["discord.com", "x.com"]);
  const [text, setText] = useState("");
  const [dialog, setDialog] = useState(false);
  const [drawer, setDrawer] = useState(false);
  const [palette, setPalette] = useState(false);
  const [page, setPage] = useState(3);
  const [step, setStep] = useState("b");

  return (
    <Stack gap={6}>
      <Heading level={1} size="2xl">Design system</Heading>

      <Block title="Buttons">
        <Stack gap={4}>
          {(["solid", "soft", "outline", "ghost"] as const).map((v) => (
            <Row key={v} gap={2} wrap>
              {TONES.map((t) => (
                <Button key={t} variant={v} tone={t}>{v} {t}</Button>
              ))}
            </Row>
          ))}
          <Row gap={2} wrap>
            <Button size="sm" icon="plus">Small</Button>
            <Button icon="download" iconRight="arrow-right">Medium</Button>
            <Button size="lg" icon="power">Large</Button>
            <Button loading>Loading</Button>
            <Button disabled>Disabled</Button>
            <IconButton icon="more" label="More" />
            <IconButton icon="trash" label="Delete" tone="danger" variant="soft" />
            <ButtonGroup>
              <Button variant="outline" tone="neutral">One</Button>
              <Button variant="outline" tone="neutral">Two</Button>
            </ButtonGroup>
          </Row>
        </Stack>
      </Block>

      <Block title="Marks, badges, stamps">
        <Stack gap={4}>
          <Row gap={4} wrap>{STATES.map((s) => <Row key={s} gap={2}><StateMark state={s} size={20} /><Text size="sm">{s}</Text></Row>)}</Row>
          <Row gap={2} wrap>{TONES.map((t) => <Badge key={t} tone={t}>{t}</Badge>)}{TONES.map((t) => <Badge key={t} tone={t} variant="outline">{t}</Badge>)}<Badge tone="accent" variant="solid" icon="bolt">solid</Badge></Row>
          <Row gap={2} wrap><Chip>discord.com</Chip><Chip tone="accent" onRemove={() => {}}>removable</Chip></Row>
          <Row gap={5} wrap><Stamp tone="ok">Active</Stamp><Stamp tone="warn" size="lg">On trial</Stamp><Stamp tone="danger" size="sm">Blocked</Stamp></Row>
          <Row gap={3}><Spinner /><Skeleton width={160} /><Kbd>Ctrl</Kbd><Kbd>K</Kbd></Row>
        </Stack>
      </Block>

      <Block title="Type">
        <Stack gap={2}>
          <Heading level={2} size="3xl">Display 3xl</Heading>
          <Heading level={2} size="2xl">Heading 2xl</Heading>
          <Heading level={3} size="lg">Heading lg</Heading>
          <Text>Body text in the interface face, with <Text mono>monospace for data</Text>.</Text>
          <Text color={2}>Secondary ink</Text>
          <Text color={3} size="sm">Tertiary ink, small</Text>
          <Text color="danger">Danger tone</Text>
        </Stack>
      </Block>

      <Block title="Form controls">
        <Grid min="16rem" gap={5}>
          <Field label="Text field" hint="Helper text"><TextField placeholder="example.com" icon="globe" /></Field>
          <Field label="With error" error="That is not a hostname"><TextField defaultValue="not a host" /></Field>
          <Field label="Search"><SearchField value={text} onChange={(e) => setText(e.currentTarget.value)} onClear={() => setText("")} placeholder="Filter" /></Field>
          <Field label="Select"><Select value={sel} onValueChange={setSel} options={[{ value: "x", label: "Option X", description: "With a description" }, { value: "y", label: "Option Y", icon: "bolt" }]} /></Field>
          <Field label="Number"><NumberField value={num} onChange={setNum} min={0} max={10} suffix="s" /></Field>
          <Field label="Slider"><Slider aria-label="Amount" value={slide} onValueChange={setSlide} format={(n) => `${n}%`} /></Field>
          <Field label="Tags"><TagInput value={tags} onChange={setTags} /></Field>
          <Field label="Text area" optional><TextArea placeholder="Notes" rows={3} /></Field>
          <Stack gap={3}>
            <Checkbox checked={check} onCheckedChange={setCheck} label="Checkbox" description="With a description" />
            <Switch checked={on} onCheckedChange={setOn} label="Switch" />
            <SegmentedControl aria-label="Segments" value={seg} onValueChange={setSeg} options={[{ value: "a", label: "One" }, { value: "b", label: "Two" }, { value: "c", label: "Three" }]} />
          </Stack>
          <RadioGroup value={radio} onValueChange={setRadio} aria-label="Radios" options={[{ value: "one", label: "One" }, { value: "two", label: "Two" }]} />
          <RadioGroup variant="cards" value={radio} onValueChange={setRadio} aria-label="Radio cards" options={[{ value: "one", label: "Card one", description: "Larger target" }, { value: "two", label: "Card two", description: "With room for content" }]} />
        </Grid>
        <Divider className="gallery-gap" />
        <FileDrop onFiles={(f) => toast({ title: `Got ${f[0].name}` })} />
      </Block>

      <Block title="Feedback">
        <Stack gap={3}>
          {TONES.map((t) => <Notice key={t} tone={t} title={`${t} notice`} action={<Button size="sm" variant="soft" tone={t}>Action</Button>}>Message text that explains what happened and what to do.</Notice>)}
          <Progress label="Determinate" value={60} />
          <Progress label="Indeterminate" indeterminate />
          <Meter label="Meter" value={72} tone="ok" />
          <Row gap={2} wrap>
            <Button variant="soft" onClick={() => toast({ title: "Saved", description: "Your config is kept.", tone: "ok" })}>Toast</Button>
            <Button variant="soft" tone="danger" onClick={() => toast({ title: "Failed", description: "Could not reach the helper.", tone: "danger" })}>Error toast</Button>
          </Row>
          <EmptyState title="Empty state" action={<Button>Do something</Button>}>Explain what will appear here.</EmptyState>
          <ErrorState error="connect ECONNREFUSED" onRetry={() => {}} />
        </Stack>
      </Block>

      <Block title="Overlays">
        <Row gap={2} wrap>
          <Button variant="outline" tone="neutral" onClick={() => setDialog(true)}>Dialog</Button>
          <Button variant="outline" tone="neutral" onClick={() => setDrawer(true)}>Drawer</Button>
          <Button variant="outline" tone="neutral" onClick={() => setPalette(true)}>Command palette</Button>
          <Button variant="outline" tone="neutral" onClick={async () => toast({ title: (await confirm({ title: "Delete it?", description: "This cannot be undone.", tone: "danger", confirmLabel: "Delete" })) ? "Deleted" : "Cancelled" })}>Confirm</Button>
          <Popover trigger={<Button variant="outline" tone="neutral">Popover</Button>}><Text size="sm">Anything can go in here.</Text></Popover>
          <Tooltip label="A tooltip"><Button variant="outline" tone="neutral">Hover me</Button></Tooltip>
          <DropdownMenu trigger={<IconButton icon="more" label="Menu" />} entries={[{ label: "Open", icon: "eye", onSelect: () => {} }, { label: "Copy", icon: "copy", shortcut: "C", onSelect: () => {} }, { type: "separator" }, { label: "Delete", icon: "trash", tone: "danger", onSelect: () => {} }]} />
        </Row>
        <Dialog open={dialog} onOpenChange={setDialog} title="Dialog" description="A modal for a task that needs focus." footer={<Button onClick={() => setDialog(false)}>Done</Button>}><Text>Body content.</Text></Dialog>
        <Drawer open={drawer} onOpenChange={setDrawer} title="Drawer" description="Detail beside the page."><Text>Body content.</Text></Drawer>
        <CommandPalette open={palette} onOpenChange={setPalette} commands={[{ id: "a", label: "First command", icon: "bolt", run: () => {} }, { id: "b", label: "Second command", icon: "sliders", hint: "hint", run: () => {} }]} />
      </Block>

      <Block title="Data">
        <Stack gap={5}>
          <KeyValue items={[{ label: "Name", value: "fake-badsum" }, { label: "Engine", value: <Text mono>nfqws2 v0.9.2</Text>, hint: "/opt/zapret2/nfqws2" }]} />
          <Table caption="Example" rows={[{ n: "alpha", v: 3 }, { n: "beta", v: 1 }, { n: "gamma", v: 2 }]} rowKey={(r) => r.n} columns={[{ key: "n", header: "Name", cell: (r) => r.n, sortValue: (r) => r.n }, { key: "v", header: "Value", cell: (r) => r.v, align: "right", sortValue: (r) => r.v, mono: true }]} />
          <RowList label="Rows" items={[{ id: "1", leading: <StateMark state="ok" />, title: "Row title", subtitle: "Row subtitle", trailing: <Badge>trailing</Badge> }, { id: "2", leading: <Icon name="layers" />, title: "Clickable row", onClick: () => {} }]} />
          <Timeline items={[{ id: "1", title: "Started", state: "ok", time: "09:12" }, { id: "2", title: "Retrying", body: "Backing off for 2s", state: "warn", time: "10:02" }]} />
          <CodeBlock title="params.txt" code={"--filter-tcp=443\n--lua-desync=fake:blob=fake_default_tls"} />
          <LogViewer height={140} text={"09:12 started\n09:40 warn: quiet for 20 minutes\n10:02 error: engine exited"} />
          <Row gap={2}><CopyButton text="hello" /><Pagination page={page} pageCount={9} onPageChange={setPage} /></Row>
          <Breadcrumbs items={[{ label: "Configs", onClick: () => {} }, { label: "fake-badsum" }]} />
        </Stack>
      </Block>

      <Block title="Navigation and disclosure">
        <Stack gap={5}>
          <Tabs items={[{ value: "a", label: "First", icon: "home", content: <Text>First panel</Text> }, { value: "b", label: "Second", content: <Text>Second panel</Text> }]} />
          <Accordion defaultOpen={["a"]} items={[{ value: "a", title: "Open section", content: <Text>Content</Text> }, { value: "b", title: "Closed section", meta: <Badge>3</Badge>, content: <Text>Content</Text> }]} />
          <Disclosure summary="Show details"><Text>Hidden until asked for.</Text></Disclosure>
          <Row gap={6} align="flex-start">
            <Steps current={step} onSelect={setStep} steps={[{ id: "a", label: "One" }, { id: "b", label: "Two", description: "Current" }, { id: "c", label: "Three" }]} />
            <Steps orientation="horizontal" current="b" steps={[{ id: "a", label: "One" }, { id: "b", label: "Two" }, { id: "c", label: "Three" }]} />
          </Row>
          <Toolbar><Button size="sm" variant="soft" tone="neutral">Toolbar</Button><Button size="sm" variant="soft" tone="neutral">Actions</Button></Toolbar>
          <ScrollArea maxHeight={80}><Stack gap={1}>{Array.from({ length: 12 }, (_, i) => <Text key={i}>Scrollable line {i + 1}</Text>)}</Stack></ScrollArea>
        </Stack>
      </Block>

      <Block title={`Icons (${iconNames.length})`}>
        <Grid min="7rem" gap={3}>
          {iconNames.map((n) => (
            <Stack key={n} gap={1} align="center">
              <Icon name={n} size={22} />
              <Text size="xs" color={3}>{n}</Text>
            </Stack>
          ))}
        </Grid>
      </Block>
    </Stack>
  );
}
