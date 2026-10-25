import { Children, isValidElement, useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ElementType, type HTMLAttributes, type ReactNode } from "react";
import { cx } from "./cx";
import { Icon, type IconName } from "./Icon";
import { Heading, Text } from "./primitives";

type Gap = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
const gapVar = (gap: Gap) => `var(--s-${gap})`;

type FlexProps = {
  as?: ElementType;
  gap?: Gap;
  align?: CSSProperties["alignItems"];
  justify?: CSSProperties["justifyContent"];
  wrap?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
} & Omit<HTMLAttributes<HTMLElement>, "style">;

export function Stack({ as: Tag = "div", gap = 3, align, justify, wrap, className, style, ...rest }: FlexProps) {
  return (
    <Tag
      className={cx("ui-flex", className)}
      style={{ flexDirection: "column", gap: gapVar(gap), alignItems: align, justifyContent: justify, flexWrap: wrap ? "wrap" : undefined, ...style }}
      {...rest}
    />
  );
}

export function Row({ as: Tag = "div", gap = 3, align = "center", justify, wrap, className, style, ...rest }: FlexProps) {
  return (
    <Tag
      className={cx("ui-flex", className)}
      style={{ flexDirection: "row", gap: gapVar(gap), alignItems: align, justifyContent: justify, flexWrap: wrap ? "wrap" : undefined, ...style }}
      {...rest}
    />
  );
}

export const Spacer = () => <div style={{ flex: 1 }} />;

/** `columns` fixes the count; `min` makes it responsive (auto-fit, each cell at least `min`). */
export function Grid({
  columns,
  min,
  gap = 4,
  align,
  className,
  style,
  children,
}: {
  columns?: number | string;
  min?: string;
  gap?: Gap;
  align?: CSSProperties["alignItems"];
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}) {
  const template = min
    ? `repeat(auto-fit, minmax(min(${min}, 100%), 1fr))`
    : typeof columns === "number"
      ? `repeat(${columns}, minmax(0, 1fr))`
      : columns;
  return (
    <div className={cx("ui-grid", className)} style={{ gridTemplateColumns: template, gap: gapVar(gap), alignItems: align, ...style }}>
      {children}
    </div>
  );
}

const toPx = (length: string) => {
  const n = parseFloat(length);
  return length.endsWith("rem") ? n * parseFloat(getComputedStyle(document.documentElement).fontSize) : n;
};

/** Fills the window down the first column and starts another only when the next item would fall
 *  below the fold, up to as many as `min` lets fit across. Items are positioned absolutely from
 *  measured heights, not moved between wrappers, so none remount on resize. Each panel is told the
 *  column height (`--cols-h`) so a long list scrolls inside its panel. */
export function Columns({ min = "24rem", gap = 5, children }: { min?: string; gap?: Gap; children?: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  const refs = useRef<(HTMLDivElement | null)[]>([]);
  const items = Children.toArray(children).filter(isValidElement);

  const layout = useCallback(() => {
    const el = box.current;
    const nodes = refs.current.slice(0, items.length).filter((n): n is HTMLDivElement => Boolean(n));
    if (!el || el.clientWidth === 0 || nodes.length === 0) return;
    const W = el.clientWidth;
    const g = toPx(getComputedStyle(el).columnGap);
    const maxCols = Math.max(1, Math.floor((W + g) / (toPx(min) + g)));

    // How much of the window is left below this container, which is what a column may use.
    const main = el.closest<HTMLElement>(".ui-main");
    let H = window.innerHeight * 0.8;
    if (main) {
      const top = el.getBoundingClientRect().top - main.getBoundingClientRect().top + main.scrollTop;
      const pad = parseFloat(getComputedStyle(main.firstElementChild ?? main).paddingBottom) || 0;
      H = main.clientHeight - top - pad;
    }
    el.style.setProperty("--cols-h", `${Math.max(H, 240)}px`);

    let chosen: { colW: number; spots: { col: number; y: number }[]; height: number } | undefined;
    for (let k = 1; k <= maxCols; k++) {
      const colW = (W - (k - 1) * g) / k;
      nodes.forEach((n) => (n.style.width = `${colW}px`));
      const heights = nodes.map((n) => n.offsetHeight);
      const spots: { col: number; y: number }[] = [];
      const bottoms: number[] = Array(k).fill(0);
      let col = 0;
      let y = 0;
      let spilled = false;
      heights.forEach((h) => {
        if (y > 0 && y + h > H) {
          if (col < k - 1) {
            col++;
            y = 0;
          } else {
            spilled = true;
          }
        }
        spots.push({ col, y });
        y += h + g;
        bottoms[col] = y - g;
      });
      chosen = { colW, spots, height: Math.max(...bottoms) };
      if (!spilled) break;
    }
    if (!chosen) return;
    nodes.forEach((n, i) => {
      n.style.left = `${chosen.spots[i].col * (chosen.colW + g)}px`;
      n.style.top = `${chosen.spots[i].y}px`;
      n.style.width = `${chosen.colW}px`;
    });
    el.style.height = `${chosen.height}px`;
  }, [min, items.length]);

  useLayoutEffect(() => {
    layout();
    const ro = new ResizeObserver(layout);
    if (box.current) ro.observe(box.current);
    refs.current.forEach((n) => n && ro.observe(n));
    window.addEventListener("resize", layout);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", layout);
    };
  }, [layout]);

  return (
    <div ref={box} className="ui-cols" style={{ columnGap: gapVar(gap) }}>
      {items.map((child, i) => (
        <div key={child.key ?? i} className="ui-cols__item" ref={(n) => void (refs.current[i] = n)}>
          {child}
        </div>
      ))}
    </div>
  );
}

type SheetProps = {
  as?: ElementType;
  variant?: "raised" | "flat" | "well";
  padding?: "none" | "sm" | "md" | "lg";
  className?: string;
  children?: ReactNode;
} & HTMLAttributes<HTMLElement>;

/** The surface everything sits on. Never nest one Sheet in another; use a Divider or `well`. */
export function Sheet({ as: Tag = "div", variant = "raised", padding = "md", className, ...rest }: SheetProps) {
  return <Tag className={cx("ui-sheet", className)} data-variant={variant} data-padding={padding} {...rest} />;
}

export function Section({
  title,
  description,
  actions,
  level = 2,
  children,
  className,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  level?: 2 | 3;
  children?: ReactNode;
  className?: string;
}) {
  return (
    <section className={cx("ui-section", className)}>
      {(title || actions) && (
        <header className="ui-section__head">
          <div className="ui-section__titles">
            {title && (
              <Heading level={level} size="lg">
                {title}
              </Heading>
            )}
            {description && (
              <Text size="sm" color={2}>
                {description}
              </Text>
            )}
          </div>
          {actions && <div className="ui-section__actions">{actions}</div>}
        </header>
      )}
      {children}
    </section>
  );
}

/** A titled surface whose body scrolls on its own, so a long list does not lengthen the page.
 *  `maxHeight` caps the body (default: what fits in the window). */
export function Panel({
  title,
  description,
  actions,
  footer,
  maxHeight,
  className,
  children,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  footer?: ReactNode;
  maxHeight?: number | string;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <section className={cx("ui-panel", className)}>
      {(title || actions) && (
        <header className="ui-panel__head">
          <div className="ui-section__titles">
            {title && (
              <Heading level={2} size="lg">
                {title}
              </Heading>
            )}
            {description && (
              <Text size="sm" color={2}>
                {description}
              </Text>
            )}
          </div>
          {actions && <div className="ui-section__actions">{actions}</div>}
        </header>
      )}
      <div className="ui-panel__body" style={{ maxHeight }}>
        {children}
      </div>
      {footer && <footer className="ui-panel__foot">{footer}</footer>}
    </section>
  );
}

export function PageHeader({
  title,
  description,
  actions,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="ui-pagehead">
      <div className="ui-pagehead__titles">
        <Heading level={1} size="2xl">
          {title}
        </Heading>
        {description && (
          <Text size="md" color={2} as="p">
            {description}
          </Text>
        )}
      </div>
      {actions && <div className="ui-pagehead__actions">{actions}</div>}
    </header>
  );
}

export function Toolbar({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cx("ui-toolbar", className)} role="toolbar">
      {children}
    </div>
  );
}

export function ScrollArea({
  maxHeight,
  height,
  className,
  children,
  ...rest
}: { maxHeight?: number | string; height?: number | string; className?: string; children: ReactNode } & HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cx("ui-scroll", className)} style={{ maxHeight, height }} {...rest}>
      {children}
    </div>
  );
}

/** Window frame: an optional titlebar row, then sidebar and a scrolling main area. Each `scrollKey`
 *  (usually the page) remembers its own scroll position. */
export function AppShell({
  titlebar,
  sidebar,
  collapsed,
  scrollKey = "",
  children,
}: {
  titlebar?: ReactNode;
  sidebar: ReactNode;
  /** Sidebar shows icons only. */
  collapsed?: boolean;
  scrollKey?: string;
  children: ReactNode;
}) {
  const main = useRef<HTMLElement>(null);
  const saved = useRef(new Map<string, number>());
  const key = useRef(scrollKey);
  // Collapsing with the pointer still over the rail must not reopen it; hover works again once the
  // pointer has left.
  const [settled, setSettled] = useState(false);
  const was = useRef(collapsed);
  useEffect(() => {
    if (collapsed && !was.current) setSettled(true);
    was.current = collapsed;
  }, [collapsed]);
  useLayoutEffect(() => {
    key.current = scrollKey;
    if (main.current) main.current.scrollTop = saved.current.get(scrollKey) ?? 0;
  }, [scrollKey]);

  return (
    <div className="ui-app">
      {titlebar}
      <div
        className="ui-shell"
        data-collapsed={collapsed || undefined}
        data-settled={settled || undefined}
        onPointerMove={(e) => settled && !(e.target as HTMLElement).closest(".ui-side") && setSettled(false)}
      >
        {sidebar}
        <main className="ui-main" id="main" ref={main} onScroll={(e) => saved.current.set(key.current, e.currentTarget.scrollTop)}>
          <div className="ui-main__inner">{children}</div>
        </main>
      </div>
    </div>
  );
}

/** The row at the top of the window. Empty space in it drags the window when running in Tauri. */
export function Titlebar({ left, right }: { left?: ReactNode; right?: ReactNode }) {
  return (
    <header className="ui-titlebar" data-tauri-drag-region>
      {left}
      <div className="ui-titlebar__fill" data-tauri-drag-region />
      {right}
    </header>
  );
}

export function Sidebar({
  brand,
  children,
  footer,
}: {
  brand?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <aside className="ui-side">
      {brand && <div className="ui-side__brand">{brand}</div>}
      <nav className="ui-side__nav" aria-label="Main">
        {children && <ul className="ui-nav">{children}</ul>}
      </nav>
      {footer && <div className="ui-side__foot">{footer}</div>}
    </aside>
  );
}

export function NavItem({
  icon,
  label,
  active,
  onClick,
  badge,
}: {
  icon: IconName;
  label: string;
  active?: boolean;
  onClick: () => void;
  badge?: ReactNode;
}) {
  return (
    <li>
      <button
        type="button"
        className="ui-nav__item"
        data-active={active || undefined}
        aria-current={active ? "page" : undefined}
        onClick={onClick}
        title={label}
      >
        <Icon name={icon} size={18} />
        <span className="ui-nav__label">{label}</span>
        {badge && <span className="ui-nav__badge">{badge}</span>}
      </button>
    </li>
  );
}

/** Collapse or expand the sidebar. Lives in the sidebar footer. */
export function SidebarToggle({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const label = collapsed ? "Expand sidebar" : "Collapse sidebar";
  return (
    <button type="button" className="ui-nav__item" onClick={onToggle} title={label} aria-label={label} aria-expanded={!collapsed}>
      <Icon name={collapsed ? "chevron-right" : "chevron-left"} size={18} />
      <span className="ui-nav__label">Collapse</span>
    </button>
  );
}
