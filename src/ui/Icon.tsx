import type { SVGProps } from "react";

/** One stroke weight, one 24px grid, round joins. Paths only; add a name here to add an icon. */
const PATHS = {
  home: "M3 11.5 12 4l9 7.5M5.5 10v9.5h13V10M10 19.5v-5h4v5",
  layers: "M12 3 3 8l9 5 9-5zM3 12.5l9 5 9-5M3 17l9 5 9-5",
  list: "M4 6h16M4 12h16M4 18h10",
  terminal: "M4 5h16v14H4zM8 10l3 2.5L8 15M13 15h4",
  shield: "M12 3l7 3v5.5c0 4.2-2.9 7.6-7 9.5-4.1-1.9-7-5.3-7-9.5V6z",
  "shield-check": "M12 3l7 3v5.5c0 4.2-2.9 7.6-7 9.5-4.1-1.9-7-5.3-7-9.5V6zM9 12l2.2 2.2L15.5 10",
  power: "M12 3v8M6.4 6.6a8 8 0 1 0 11.2 0",
  play: "M7 5v14l12-7z",
  pause: "M8 5v14M16 5v14",
  stop: "M6 6h12v12H6z",
  refresh: "M20 11a8 8 0 0 0-14.5-3.5L4 9M4 4v5h5M4 13a8 8 0 0 0 14.5 3.5L20 15M20 20v-5h-5",
  check: "M5 12.5l4.5 4.5L19 7.5",
  x: "M6 6l12 12M18 6L6 18",
  alert: "M12 4l9 16H3zM12 10v4.5M12 17.5v.6",
  info: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 11v5M12 8v.6",
  help: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.4-1 .9-1 1.7M12 17v.6",
  "chevron-right": "M9 6l6 6-6 6",
  "chevron-down": "M6 9l6 6 6-6",
  "chevron-left": "M15 6l-6 6 6 6",
  "chevron-up": "M6 15l6-6 6 6",
  "chevrons-vertical": "M8 9l4-4 4 4M8 15l4 4 4-4",
  "arrow-right": "M5 12h14M13 6l6 6-6 6",
  "arrow-left": "M19 12H5M11 6l-6 6 6 6",
  "arrow-up": "M12 19V5M6 11l6-6 6 6",
  "arrow-down": "M12 5v14M6 13l6 6 6-6",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4",
  copy: "M9 9h10v11H9zM5 15V4h10",
  trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
  download: "M12 4v11M7 11l5 5 5-5M5 20h14",
  upload: "M12 16V5M7 9l5-5 5 5M5 20h14",
  external: "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
  sliders: "M4 7h9M17 7h3M4 17h3M11 17h9M15 4v6M9 14v6",
  sun: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4",
  moon: "M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z",
  monitor: "M3 5h18v12H3zM8 21h8M12 17v4",
  plus: "M12 5v14M5 12h14",
  minus: "M5 12h14",
  filter: "M4 5h16l-6 8v6l-4-2v-4z",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2",
  history: "M4 12a8 8 0 1 0 2.5-5.8L4 8M4 4v4h4M12 8v4l3 2",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18",
  server: "M4 4h16v6H4zM4 14h16v6H4zM8 7v.6M8 17v.6",
  cpu: "M7 7h10v10H7zM10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4",
  lock: "M6 11h12v9H6zM8 11V8a4 4 0 0 1 8 0v3",
  unlock: "M6 11h12v9H6zM8 11V8a4 4 0 0 1 7.5-2",
  file: "M6 3h8l4 4v14H6zM14 3v4h4",
  folder: "M3 6h6l2 2h10v11H3z",
  eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
  "eye-off":
    "M3 3l18 18M10.6 6.1A9 9 0 0 1 12 5c6 0 10 7 10 7a17 17 0 0 1-3 3.8M6.5 7.5A17 17 0 0 0 2 12s4 7 10 7c1.6 0 3-.4 4.3-1M9.9 9.9a3 3 0 0 0 4.2 4.2",
  more: "M4 12a1.2 1.2 0 1 0 2.4 0 1.2 1.2 0 1 0-2.4 0M10.8 12a1.2 1.2 0 1 0 2.4 0 1.2 1.2 0 1 0-2.4 0M17.6 12a1.2 1.2 0 1 0 2.4 0 1.2 1.2 0 1 0-2.4 0",
  undo: "M9 14L4 9l5-5M4 9h10a6 6 0 0 1 0 12h-3",
  bolt: "M13 3L5 14h6l-1 7 8-11h-6z",
  wifi: "M2 9a15 15 0 0 1 20 0M5 12.5a10.5 10.5 0 0 1 14 0M8.5 16a5.5 5.5 0 0 1 7 0M12 19.5v.6",
  "wifi-off":
    "M3 3l18 18M2 9a15 15 0 0 1 5-3M22 9a15 15 0 0 0-8-3.8M5 12.5a10.5 10.5 0 0 1 4-2.4M19 12.5a10.5 10.5 0 0 0-2.5-1.8M8.5 16a5.5 5.5 0 0 1 7 0M12 19.5v.6",
  bug: "M9 7a3 3 0 0 1 6 0M8 8h8v6a4 4 0 0 1-8 0zM4 10h4M16 10h4M4 18l4-2M20 18l-4-2M12 8v10",
  edit: "M4 20h4L19 9l-4-4L4 16zM13 7l4 4",
  save: "M5 4h11l3 3v13H5zM8 4v5h7V4M8 20v-6h8v6",
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  wand: "M12 3l2 5 5 2-5 2-2 5-2-5-5-2 5-2zM19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z",
  command:
    "M9 9H6a3 3 0 1 1 3-3zM9 9h6M15 9V6a3 3 0 1 1 3 3zM15 9v6M15 15h3a3 3 0 1 1-3 3zM15 15H9M9 15v3a3 3 0 1 1-3-3zM9 15V9",
  route:
    "M6 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM18 9a2 2 0 1 0 0-4 2 2 0 0 0 0 4zM8 17h6a3 3 0 0 0 0-6h-4a3 3 0 0 1 0-6h6",
  sort: "M8 4v16M4 8l4-4 4 4M16 20V4M12 16l4 4 4-4",
  grip: "M9 6h.6M15 6h.6M9 12h.6M15 12h.6M9 18h.6M15 18h.6",
  sidebar: "M3 5h18v14H3zM9 5v14",
  package: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM4 7.5l8 4.5 8-4.5M12 12v9",
  toggle: "M8 7h8a5 5 0 0 1 0 10H8A5 5 0 0 1 8 7zM8 9.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z",
  zap: "M13 2L4 14h7l-1 8 9-12h-7z",
  heart: "M12 20s-8-5-8-11a4.5 4.5 0 0 1 8-2.8A4.5 4.5 0 0 1 20 9c0 6-8 11-8 11z",
  user: "M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM4 20a8 8 0 0 1 16 0",
  calendar: "M4 6h16v14H4zM4 10h16M8 3v4M16 3v4",
  tag: "M3 12V4h8l10 10-8 8zM7.5 8v.6",
  maximize: "M5.5 5.5h13v13h-13z",
  restore: "M9 9h9.5v9.5H9zM5.5 15V5.5H15",
  star: "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z",
} as const;

export type IconName = keyof typeof PATHS;
export const iconNames = Object.keys(PATHS) as IconName[];

type Props = { name: IconName; size?: number; strokeWidth?: number } & Omit<
  SVGProps<SVGSVGElement>,
  "name"
>;

/** Decorative by default (aria-hidden); pass `aria-label` to make it meaningful. */
export function Icon({ name, size = 18, strokeWidth = 1.7, ...rest }: Props) {
  const labelled = rest["aria-label"] !== undefined;
  return (
    <svg
      className="ui-icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={labelled ? undefined : true}
      role={labelled ? "img" : undefined}
      focusable="false"
      {...rest}
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
