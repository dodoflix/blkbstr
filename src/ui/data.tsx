import { useLayoutEffect, useMemo, useRef, useState, type ReactNode, type UIEvent } from "react";
import { Accordion as RAccordion, Collapsible as RCollapsible, Tabs as RTabs } from "radix-ui";
import { cx } from "./cx";
import { useClipboard } from "./hooks";
import { Icon, type IconName } from "./Icon";
import { StateMark } from "./marks";
import { Button, IconButton, Text } from "./primitives";
import { EmptyState } from "./feedback";
import type { State } from "./types";

export type KeyValueItem = { label: ReactNode; value: ReactNode; hint?: ReactNode; hidden?: boolean };

/** Label/value rows with a hairline between them. */
export function KeyValue({ items, className }: { items: KeyValueItem[]; className?: string }) {
  return (
    <dl className={cx("ui-kv", className)}>
      {items
        .filter((i) => !i.hidden)
        .map((item, i) => (
          <div className="ui-kv__row" key={i}>
            <dt>{item.label}</dt>
            <dd>
              {item.value}
              {item.hint && (
                <Text as="div" size="sm" color={3}>
                  {item.hint}
                </Text>
              )}
            </dd>
          </div>
        ))}
    </dl>
  );
}

export type Column<T> = {
  key: string;
  header: ReactNode;
  cell: (row: T) => ReactNode;
  align?: "left" | "right" | "center";
  width?: string;
  /** Present means the column can be sorted by this value. */
  sortValue?: (row: T) => string | number;
  mono?: boolean;
};

export function Table<T>({
  columns,
  rows,
  rowKey,
  onRowClick,
  empty,
  dense,
  initialSort,
  caption,
}: {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
  onRowClick?: (row: T) => void;
  empty?: ReactNode;
  dense?: boolean;
  initialSort?: { key: string; dir: "asc" | "desc" };
  caption: string;
}) {
  const [sort, setSort] = useState(initialSort);
  const sorted = useMemo(() => {
    const col = columns.find((c) => c.key === sort?.key);
    if (!col?.sortValue || !sort) return rows;
    const get = col.sortValue;
    const dir = sort.dir === "asc" ? 1 : -1;
    return [...rows].sort((a, b) => {
      const x = get(a);
      const y = get(b);
      return (x < y ? -1 : x > y ? 1 : 0) * dir;
    });
  }, [rows, columns, sort]);

  if (rows.length === 0 && empty) return <>{empty}</>;
  return (
    <div className="ui-tablewrap">
      <table className="ui-table" data-dense={dense || undefined}>
        <caption className="ui-sr-only">{caption}</caption>
        <thead>
          <tr>
            {columns.map((c) => {
              const active = sort?.key === c.key;
              return (
                <th
                  key={c.key}
                  scope="col"
                  style={{ width: c.width, textAlign: c.align }}
                  aria-sort={active ? (sort?.dir === "asc" ? "ascending" : "descending") : undefined}
                >
                  {c.sortValue ? (
                    <button
                      type="button"
                      className="ui-table__sort"
                      onClick={() => setSort({ key: c.key, dir: active && sort?.dir === "asc" ? "desc" : "asc" })}
                    >
                      {c.header}
                      <Icon name={active ? (sort?.dir === "asc" ? "arrow-up" : "arrow-down") : "sort"} size={13} />
                    </button>
                  ) : (
                    c.header
                  )}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => (
            <tr
              key={rowKey(row)}
              data-clickable={onRowClick || undefined}
              onClick={onRowClick ? () => onRowClick(row) : undefined}
              onKeyDown={onRowClick ? (e) => e.key === "Enter" && onRowClick(row) : undefined}
              tabIndex={onRowClick ? 0 : undefined}
            >
              {columns.map((c) => (
                <td key={c.key} style={{ textAlign: c.align }} className={cx(c.mono && "ui-table__mono")}>
                  {c.cell(row)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function CopyButton({ text, label = "Copy", size = "sm" }: { text: string; label?: string; size?: "sm" | "md" }) {
  const { copied, copy } = useClipboard();
  return (
    <Button size={size} variant="soft" tone={copied ? "ok" : "neutral"} icon={copied ? "check" : "copy"} onClick={() => void copy(text)}>
      {copied ? "Copied" : label}
    </Button>
  );
}

/** Preformatted text for parameter files, commands and JSON. */
export function CodeBlock({
  code,
  title,
  maxHeight = 320,
  wrap,
  copyable = true,
}: {
  code: string;
  title?: ReactNode;
  maxHeight?: number | string;
  wrap?: boolean;
  copyable?: boolean;
}) {
  return (
    <figure className="ui-code">
      {(title || copyable) && (
        <figcaption className="ui-code__bar">
          <Text size="sm" color={2} weight="medium">
            {title}
          </Text>
          {copyable && <CopyButton text={code} />}
        </figcaption>
      )}
      <pre className="ui-code__pre" data-wrap={wrap || undefined} style={{ maxHeight }} tabIndex={0}>
        <code>{code}</code>
      </pre>
    </figure>
  );
}

type Level = "error" | "warn" | "info" | "debug";
const LEVEL_RULES: [RegExp, Level][] = [
  [/\b(error|fatal|panic|failed|refused)\b/i, "error"],
  [/\b(warn|warning)\b/i, "warn"],
  [/\bdebug|trace\b/i, "debug"],
];
const levelOf = (line: string): Level => LEVEL_RULES.find(([re]) => re.test(line))?.[1] ?? "info";

const LOG_ROW = 20;
const LOG_OVERSCAN = 24;

/** Reads a log as text with level colouring, line numbers, a substring filter and follow-the-tail.
 *  Only the rows in view are rendered (thousands of lines made the page stutter). Scrolling up pauses
 *  following and reaching the bottom resumes it, via `onFollowChange`. */
export function LogViewer({
  text,
  filter = "",
  follow,
  onFollowChange,
  height = 420,
  empty = "(nothing matches)",
}: {
  text: string;
  filter?: string;
  follow?: boolean;
  onFollowChange?: (follow: boolean) => void;
  height?: number | string;
  empty?: ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [top, setTop] = useState(0);
  const [view, setView] = useState(400);
  const lines = useMemo(() => {
    const needle = filter.toLowerCase();
    return text
      .split("\n")
      .map((line, n) => ({ n: n + 1, line }))
      .filter(({ line }) => line.length > 0 && (!needle || line.toLowerCase().includes(needle)));
  }, [text, filter]);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setView(el.clientHeight);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useLayoutEffect(() => {
    const el = ref.current;
    if (follow && el) {
      el.scrollTop = el.scrollHeight;
      setTop(el.scrollTop);
    }
  }, [lines, follow]);

  const onScroll = (e: UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    setTop(el.scrollTop);
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < LOG_ROW;
    if (follow && !atBottom) onFollowChange?.(false);
    else if (!follow && atBottom) onFollowChange?.(true);
  };

  const first = Math.max(0, Math.floor(top / LOG_ROW) - LOG_OVERSCAN);
  const last = Math.min(lines.length, Math.ceil((top + view) / LOG_ROW) + LOG_OVERSCAN);

  return (
    <div className="ui-log" ref={ref} style={{ height }} onScroll={onScroll} tabIndex={0} role="log" aria-label="Log output">
      {lines.length === 0 ? (
        <div className="ui-log__empty">{empty}</div>
      ) : (
        <div style={{ height: lines.length * LOG_ROW, position: "relative" }}>
          <div style={{ transform: `translateY(${first * LOG_ROW}px)` }}>
            {lines.slice(first, last).map(({ n, line }) => (
              <div className="ui-log__line" data-level={levelOf(line)} key={n}>
                <span className="ui-log__n">{n}</span>
                <span className="ui-log__t">{line}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export type TabItem = { value: string; label: ReactNode; icon?: IconName; content: ReactNode; badge?: ReactNode };

export function Tabs({
  items,
  value,
  onValueChange,
  defaultValue,
}: {
  items: TabItem[];
  value?: string;
  onValueChange?: (value: string) => void;
  defaultValue?: string;
}) {
  return (
    <RTabs.Root className="ui-tabs" value={value} onValueChange={onValueChange} defaultValue={defaultValue ?? items[0]?.value}>
      <RTabs.List className="ui-tabs__list">
        {items.map((t) => (
          <RTabs.Trigger key={t.value} value={t.value} className="ui-tabs__tab">
            {t.icon && <Icon name={t.icon} size={16} />}
            {t.label}
            {t.badge}
          </RTabs.Trigger>
        ))}
      </RTabs.List>
      {items.map((t) => (
        <RTabs.Content key={t.value} value={t.value} className="ui-tabs__panel">
          {t.content}
        </RTabs.Content>
      ))}
    </RTabs.Root>
  );
}

export type AccordionItem = { value: string; title: ReactNode; meta?: ReactNode; content: ReactNode };

export function Accordion({ items, type = "multiple", defaultOpen = [] }: { items: AccordionItem[]; type?: "single" | "multiple"; defaultOpen?: string[] }) {
  const common = { className: "ui-acc" };
  const body = items.map((it) => (
    <RAccordion.Item key={it.value} value={it.value} className="ui-acc__item">
      <RAccordion.Header asChild>
        <h3 className="ui-acc__head">
          <RAccordion.Trigger className="ui-acc__trigger">
            <span className="ui-acc__title">{it.title}</span>
            {it.meta}
            <Icon name="chevron-down" size={16} />
          </RAccordion.Trigger>
        </h3>
      </RAccordion.Header>
      <RAccordion.Content className="ui-acc__content">
        <div className="ui-acc__inner">{it.content}</div>
      </RAccordion.Content>
    </RAccordion.Item>
  ));
  return type === "single" ? (
    <RAccordion.Root type="single" collapsible defaultValue={defaultOpen[0]} {...common}>
      {body}
    </RAccordion.Root>
  ) : (
    <RAccordion.Root type="multiple" defaultValue={defaultOpen} {...common}>
      {body}
    </RAccordion.Root>
  );
}

/** One disclosure; for "Show details" style reveals. */
export function Disclosure({ summary, children, defaultOpen }: { summary: ReactNode; children: ReactNode; defaultOpen?: boolean }) {
  return (
    <RCollapsible.Root className="ui-disc" defaultOpen={defaultOpen}>
      <RCollapsible.Trigger className="ui-disc__summary">
        <Icon name="chevron-right" size={15} />
        {summary}
      </RCollapsible.Trigger>
      <RCollapsible.Content className="ui-disc__content">
        <div className="ui-disc__body">{children}</div>
      </RCollapsible.Content>
    </RCollapsible.Root>
  );
}

export function Pagination({ page, pageCount, onPageChange }: { page: number; pageCount: number; onPageChange: (page: number) => void }) {
  if (pageCount <= 1) return null;
  const pages = [...new Set([1, page - 1, page, page + 1, pageCount])].filter((p) => p >= 1 && p <= pageCount).sort((a, b) => a - b);
  return (
    <nav className="ui-pager" aria-label="Pagination">
      <IconButton icon="chevron-left" label="Previous page" size="sm" disabled={page <= 1} onClick={() => onPageChange(page - 1)} />
      {pages.map((p, i) => (
        <span key={p} className="ui-pager__slot">
          {i > 0 && p - pages[i - 1] > 1 && <span className="ui-pager__gap">...</span>}
          <Button size="sm" variant={p === page ? "solid" : "ghost"} tone={p === page ? "accent" : "neutral"} aria-current={p === page ? "page" : undefined} onClick={() => onPageChange(p)}>
            {p}
          </Button>
        </span>
      ))}
      <IconButton icon="chevron-right" label="Next page" size="sm" disabled={page >= pageCount} onClick={() => onPageChange(page + 1)} />
    </nav>
  );
}

export type TimelineItem = { id: string; title: ReactNode; body?: ReactNode; time?: ReactNode; state?: State };

export function Timeline({ items, empty }: { items: TimelineItem[]; empty?: ReactNode }) {
  if (items.length === 0) return <>{empty ?? <EmptyState title="Nothing yet" />}</>;
  return (
    <ol className="ui-timeline">
      {items.map((it) => (
        <li key={it.id} className="ui-timeline__item">
          <StateMark state={it.state ?? "idle"} />
          <div className="ui-timeline__main">
            <div className="ui-timeline__title">{it.title}</div>
            {it.body && (
              <Text as="div" size="sm" color={2}>
                {it.body}
              </Text>
            )}
          </div>
          {it.time && (
            <Text size="sm" color={3} mono>
              {it.time}
            </Text>
          )}
        </li>
      ))}
    </ol>
  );
}

export function Breadcrumbs({ items }: { items: { label: string; onClick?: () => void }[] }) {
  return (
    <nav aria-label="Breadcrumb" className="ui-crumbs">
      {items.map((it, i) => (
        <span key={i} className="ui-crumbs__item">
          {i > 0 && <Icon name="chevron-right" size={13} />}
          {it.onClick && i < items.length - 1 ? (
            <button type="button" className="ui-crumbs__btn" onClick={it.onClick}>
              {it.label}
            </button>
          ) : (
            <span aria-current={i === items.length - 1 ? "page" : undefined}>{it.label}</span>
          )}
        </span>
      ))}
    </nav>
  );
}

export type RowItem = {
  id: string;
  leading?: ReactNode;
  title: ReactNode;
  subtitle?: ReactNode;
  trailing?: ReactNode;
  onClick?: () => void;
  /** Marks the row that is currently shown elsewhere (a master-detail list). */
  active?: boolean;
};

/** A vertical list of rows: mark or icon, two lines of text, something on the right. */
export function RowList({ items, label, empty }: { items: RowItem[]; label: string; empty?: ReactNode }) {
  if (items.length === 0) return <>{empty}</>;
  return (
    <ul className="ui-rows" aria-label={label}>
      {items.map((it) => {
        // The click target is the text only, so buttons in `trailing` are not nested in it.
        const body = (
          <>
            {it.leading && <span className="ui-rows__lead">{it.leading}</span>}
            <span className="ui-rows__main">
              <span className="ui-rows__title">{it.title}</span>
              {it.subtitle && <span className="ui-rows__sub">{it.subtitle}</span>}
            </span>
          </>
        );
        return (
          <li key={it.id} className="ui-rows__item" data-active={it.active || undefined}>
            <div className="ui-rows__row">
              {it.onClick ? (
                <button type="button" className="ui-rows__hit" onClick={it.onClick}>
                  {body}
                </button>
              ) : (
                <div className="ui-rows__hit">{body}</div>
              )}
              {it.trailing && <span className="ui-rows__trail">{it.trailing}</span>}
            </div>
          </li>
        );
      })}
    </ul>
  );
}
