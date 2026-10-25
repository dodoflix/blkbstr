# Design system

`src/ui` holds everything visual. A screen in `src/features` imports from `src/ui` and writes no
colours, radii, shadows or durations of its own.

## Layout of `src/ui`

| Path | Holds |
| --- | --- |
| `styles/tokens.css` | Colour, type, space, shape, motion. The only place a raw value lives |
| `styles/*.css` | One file per component group, reading tokens only |
| `theme.tsx` | `ThemeProvider`, `useAppearance`, `AppearanceForm` |
| `Icon.tsx` | One 24px stroke icon set; add a name to the map to add an icon |
| `primitives.tsx`, `marks.tsx`, `layout.tsx`, `forms.tsx`, `feedback.tsx`, `overlay.tsx`, `data.tsx` | Components |
| `hooks/` | Storage, async, polling, hotkeys, clipboard, media queries |
| `Gallery.tsx` | Every component, live. Shown on the Developer page |

## Rules

- One shape language: rounded squares. No circles, no pills.
- Flat surfaces separated by a 1px line. Only things that float (menus, dialogs, toasts) get a shadow.
- State is shape first and colour second (`StateMark`); tone alone never carries meaning.
- Motion is short and rare: fades on overlays, one stamp landing on a saved config, functional
  progress. Nothing animates on page change.
- Appearance is two data attributes on `<html>`: `data-theme` (resolved to light or dark) and
  `data-motion`. The accent is not a choice: near-black on light, white on dark. Dark is neutral
  black, not tinted.
- Pages fill the window. Content sits in `Columns`, which fills down the first column and starts another only when the next
  item would fall below the fold (up to as many as fit across), and
  anything that can grow long (site lists, logs, attempts) lives in a `Panel` whose body scrolls on
  its own, so the page itself stays short and the rest stays in view.
- Anything that loads reserves its final size (skeletons match the loaded footprint), so nothing
  shifts when data arrives.

## Dev without the backend

`npm run dev` in a plain browser answers every command from `src/mock.ts`. Pick a scenario with
`?mock=fresh|ready|nodaemon|noengine`. The mock is excluded from production builds.

## Developer page

Dev builds (`npm run dev`, `tauri dev`) add a Developer item to the sidebar. It is compiled out of
release builds: the page is behind `import.meta.env.DEV` and a lazy import, so its code never ships.

- **Components**: every component in `src/ui`.
- **Tokens**: colours, computed contrast ratios, type, space, shape, shadow and motion durations,
  recomputed when the appearance changes.
- **Screens**: the real screens, inert, over fixed fake backend state (`src/features/dev/fake.tsx`),
  so situations such as no daemon, a trial run or a down network can be looked at on demand.
- **Window sizes**: the live app in an iframe of an exact size, which is the only way to exercise the
  media queries.

The bar at the top switches theme and motion for all four. A regression pass is each tab in light
and dark, and with reduced motion.
