import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";
import { Dialog as RDialog, DropdownMenu as RMenu, Popover as RPopover, Tooltip as RTooltip } from "radix-ui";
import { cx } from "./cx";
import { Icon, type IconName } from "./Icon";
import { Button, IconButton, Kbd, Text } from "./primitives";
import { SearchField } from "./forms";
import type { Tone } from "./types";

export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  size = "md",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  size?: "sm" | "md" | "lg";
}) {
  return (
    <RDialog.Root open={open} onOpenChange={onOpenChange}>
      <RDialog.Portal>
        <RDialog.Overlay className="ui-scrim" />
        <RDialog.Content className="ui-dialog" data-size={size}>
          <header className="ui-dialog__head">
            <div>
              <RDialog.Title className="ui-dialog__title">{title}</RDialog.Title>
              {description ? (
                <RDialog.Description className="ui-dialog__desc">{description}</RDialog.Description>
              ) : (
                <RDialog.Description className="ui-sr-only">{title}</RDialog.Description>
              )}
            </div>
            <RDialog.Close asChild>
              <IconButton icon="x" label="Close" size="sm" />
            </RDialog.Close>
          </header>
          {children && <div className="ui-dialog__body">{children}</div>}
          {footer && <footer className="ui-dialog__foot">{footer}</footer>}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}

/** A side sheet for detail that should not lose the page behind it. */
export function Drawer({
  open,
  onOpenChange,
  title,
  description,
  children,
  footer,
  side = "right",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  side?: "right" | "left";
}) {
  return (
    <RDialog.Root open={open} onOpenChange={onOpenChange}>
      <RDialog.Portal>
        <RDialog.Overlay className="ui-scrim" />
        <RDialog.Content className="ui-drawer" data-side={side}>
          <header className="ui-dialog__head">
            <div>
              <RDialog.Title className="ui-dialog__title">{title}</RDialog.Title>
              {description ? (
                <RDialog.Description className="ui-dialog__desc">{description}</RDialog.Description>
              ) : (
                <RDialog.Description className="ui-sr-only">{title}</RDialog.Description>
              )}
            </div>
            <RDialog.Close asChild>
              <IconButton icon="x" label="Close" size="sm" />
            </RDialog.Close>
          </header>
          <div className="ui-drawer__body">{children}</div>
          {footer && <footer className="ui-dialog__foot">{footer}</footer>}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}

export type ConfirmOptions = {
  title: ReactNode;
  description?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: Tone;
};

const ConfirmContext = createContext<((options: ConfirmOptions) => Promise<boolean>) | null>(null);

/** `const confirm = useConfirm(); if (await confirm({...})) ...` */
export function useConfirm() {
  const confirm = useContext(ConfirmContext);
  if (!confirm) throw new Error("useConfirm needs a ConfirmProvider");
  return confirm;
}

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const [pending, setPending] = useState<{ options: ConfirmOptions; resolve: (ok: boolean) => void }>();
  const confirm = useCallback(
    (options: ConfirmOptions) => new Promise<boolean>((resolve) => setPending({ options, resolve })),
    [],
  );
  const settle = (ok: boolean) => {
    pending?.resolve(ok);
    setPending(undefined);
  };
  const o = pending?.options;
  return (
    <ConfirmContext.Provider value={confirm}>
      {children}
      <Dialog
        open={Boolean(pending)}
        onOpenChange={(open) => !open && settle(false)}
        title={o?.title}
        description={o?.description}
        size="sm"
        footer={
          <>
            <Button variant="ghost" tone="neutral" onClick={() => settle(false)}>
              {o?.cancelLabel ?? "Cancel"}
            </Button>
            <Button tone={o?.tone ?? "accent"} onClick={() => settle(true)}>
              {o?.confirmLabel ?? "Confirm"}
            </Button>
          </>
        }
      />
    </ConfirmContext.Provider>
  );
}

export function Popover({
  trigger,
  children,
  align = "start",
  width,
}: {
  trigger: ReactElement;
  children: ReactNode;
  align?: "start" | "center" | "end";
  width?: number | string;
}) {
  return (
    <RPopover.Root>
      <RPopover.Trigger asChild>{trigger}</RPopover.Trigger>
      <RPopover.Portal>
        <RPopover.Content className="ui-menu ui-popover" sideOffset={6} align={align} style={{ width }}>
          {children}
        </RPopover.Content>
      </RPopover.Portal>
    </RPopover.Root>
  );
}

export function TooltipProvider({ children }: { children: ReactNode }) {
  return <RTooltip.Provider delayDuration={350}>{children}</RTooltip.Provider>;
}

export function Tooltip({ label, children, side = "top" }: { label: ReactNode; children: ReactElement; side?: "top" | "right" | "bottom" | "left" }) {
  return (
    <RTooltip.Root>
      <RTooltip.Trigger asChild>{children}</RTooltip.Trigger>
      <RTooltip.Portal>
        <RTooltip.Content className="ui-tooltip" side={side} sideOffset={6}>
          {label}
        </RTooltip.Content>
      </RTooltip.Portal>
    </RTooltip.Root>
  );
}

export type MenuEntry =
  | { type?: "item"; label: ReactNode; icon?: IconName; onSelect: () => void; tone?: "danger"; disabled?: boolean; shortcut?: string }
  | { type: "separator" }
  | { type: "label"; label: ReactNode };

export function DropdownMenu({ trigger, entries, align = "end" }: { trigger: ReactElement; entries: MenuEntry[]; align?: "start" | "center" | "end" }) {
  return (
    <RMenu.Root>
      <RMenu.Trigger asChild>{trigger}</RMenu.Trigger>
      <RMenu.Portal>
        <RMenu.Content className="ui-menu ui-menu--plain" sideOffset={6} align={align}>
          {entries.map((e, i) => {
            if (e.type === "separator") return <RMenu.Separator key={i} className="ui-menu__sep" />;
            if (e.type === "label") return <RMenu.Label key={i} className="ui-menu__label">{e.label}</RMenu.Label>;
            return (
              <RMenu.Item key={i} className="ui-menu__item" data-tone={e.tone} disabled={e.disabled} onSelect={e.onSelect}>
                {e.icon && <Icon name={e.icon} size={16} />}
                <span className="ui-menu__text">{e.label}</span>
                {e.shortcut && <Kbd>{e.shortcut}</Kbd>}
              </RMenu.Item>
            );
          })}
        </RMenu.Content>
      </RMenu.Portal>
    </RMenu.Root>
  );
}

export type Command = {
  id: string;
  label: string;
  hint?: string;
  icon?: IconName;
  keywords?: string;
  /** Leave the palette open after running, for commands that are toggles. */
  keepOpen?: boolean;
  run: () => void;
};

/** Ctrl/Cmd+K launcher. Pass the commands; it owns open state, filtering and arrow-key movement. */
export function CommandPalette({ commands, open, onOpenChange }: { commands: Command[]; open: boolean; onOpenChange: (open: boolean) => void }) {
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const list = useRef<HTMLUListElement>(null);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? commands.filter((c) => `${c.label} ${c.keywords ?? ""}`.toLowerCase().includes(q)) : commands;
  }, [commands, query]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setIndex(0);
    }
  }, [open]);
  useEffect(() => setIndex(0), [query]);
  useEffect(() => {
    list.current?.children[index]?.scrollIntoView({ block: "nearest" });
  }, [index]);

  const run = (c?: Command) => {
    if (!c) return;
    if (!c.keepOpen) onOpenChange(false);
    c.run();
  };

  return (
    <RDialog.Root open={open} onOpenChange={onOpenChange}>
      <RDialog.Portal>
        <RDialog.Overlay className="ui-scrim" />
        <RDialog.Content
          className="ui-dialog ui-cmd"
          data-size="md"
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setIndex((i) => Math.min(results.length - 1, i + 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setIndex((i) => Math.max(0, i - 1));
            } else if (e.key === "Enter") {
              e.preventDefault();
              run(results[index]);
            }
          }}
        >
          <RDialog.Title className="ui-sr-only">Command palette</RDialog.Title>
          <RDialog.Description className="ui-sr-only">Type to filter, arrow keys to move, Enter to run.</RDialog.Description>
          <div className="ui-cmd__input">
            <SearchField value={query} onChange={(e) => setQuery(e.currentTarget.value)} placeholder="Type a command or search..." size="lg" aria-label="Command" autoFocus />
          </div>
          <ul className="ui-cmd__list" ref={list} role="listbox">
            {results.length === 0 && (
              <li className="ui-cmd__empty">
                <Text color={3}>Nothing matches "{query}".</Text>
              </li>
            )}
            {results.map((c, i) => (
              <li key={c.id} role="option" aria-selected={i === index}>
                <button type="button" className={cx("ui-cmd__item")} data-active={i === index || undefined} onMouseMove={() => setIndex(i)} onClick={() => run(c)}>
                  {c.icon && <Icon name={c.icon} size={17} />}
                  <span className="ui-cmd__label">{c.label}</span>
                  {c.hint && <Text size="sm" color={3}>{c.hint}</Text>}
                </button>
              </li>
            ))}
          </ul>
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}
