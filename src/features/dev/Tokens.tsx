import { useLayoutEffect, useState } from "react";
import { Button, Grid, Row, Section, Sheet, Stack, Text, useAppearance } from "../../ui";

const COLORS: [string, string[]][] = [
  ["Surfaces", ["paper-0", "paper-1", "paper-2", "paper-3", "line", "line-strong"]],
  ["Ink", ["ink-1", "ink-2", "ink-3", "ink-inverse"]],
  ["Accent", ["accent", "accent-hover", "accent-soft", "accent-line", "on-accent"]],
  ["Meaning", ["ok", "ok-soft", "warn", "warn-soft", "danger", "danger-soft", "on-solid"]],
];

const PAIRS: [string, string, number][] = [
  ["ink-1", "paper-0", 4.5],
  ["ink-1", "paper-1", 4.5],
  ["ink-2", "paper-0", 4.5],
  ["ink-2", "paper-1", 4.5],
  ["ink-3", "paper-0", 4.5],
  ["ink-3", "paper-1", 4.5],
  ["accent", "paper-0", 4.5],
  ["accent", "accent-soft", 4.5],
  ["on-accent", "accent", 4.5],
  ["ok", "paper-0", 4.5],
  ["warn", "paper-0", 4.5],
  ["danger", "paper-0", 4.5],
];

const SIZES = ["t-xs", "t-sm", "t-md", "t-lg", "t-xl", "t-2xl", "t-3xl"];
const SPACE = ["s-1", "s-2", "s-3", "s-4", "s-5", "s-6", "s-7", "s-8"];
const RADII = ["r-1", "r-2", "r-3", "r-4"];
const DURATIONS = ["dur-1", "dur-2", "dur-3"];

const read = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(`--${name}`).trim();

/** Resolves a token to rgb by letting the browser compute it on a probe element. */
function rgb(name: string): [number, number, number, number] {
  const probe = document.createElement("span");
  probe.style.color = `var(--${name})`;
  document.body.appendChild(probe);
  const m = getComputedStyle(probe).color.match(/[\d.]+/g)?.map(Number) ?? [0, 0, 0, 1];
  probe.remove();
  return [m[0], m[1], m[2], m[3] ?? 1];
}

const lum = ([r, g, b]: number[]) => {
  const f = (c: number) => ((c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
};

/** Composites `fg` over `bg` so translucent tokens (dark-theme softs) are judged as seen. */
function contrast(fg: string, bg: string): number {
  const b = rgb(bg);
  const f = rgb(fg);
  const mixed = f.slice(0, 3).map((c, i) => c * f[3] + b[i] * (1 - f[3]));
  const [a, z] = [lum(mixed), lum(b)];
  return (Math.max(a, z) + 0.05) / (Math.min(a, z) + 0.05);
}

export function Tokens() {
  const a = useAppearance();
  const [values, setValues] = useState<Record<string, string>>({});
  const [ratios, setRatios] = useState<number[]>([]);
  const [fired, setFired] = useState(0);

  // Recomputed whenever the appearance changes, because every value here depends on it.
  useLayoutEffect(() => {
    const names = [...COLORS.flatMap(([, n]) => n), ...SIZES, ...SPACE, ...RADII, ...DURATIONS];
    requestAnimationFrame(() => {
      setValues(Object.fromEntries(names.map((n) => [n, read(n)])));
      setRatios(PAIRS.map(([f, b]) => contrast(f, b)));
    });
  }, [a.resolved, a.motion]);

  return (
    <Stack gap={6}>
      {COLORS.map(([title, names]) => (
        <Section key={title} title={title}>
          <Sheet>
            <Grid min="9rem" gap={3}>
              {names.map((n) => (
                <Stack key={n} gap={1}>
                  <div className="dev-swatch" style={{ background: `var(--${n})` }} />
                  <Text size="sm" weight="semibold">{n}</Text>
                  <Text size="xs" color={3} mono>{values[n]}</Text>
                </Stack>
              ))}
            </Grid>
          </Sheet>
        </Section>
      ))}

      <Section title="Contrast" description="Computed from the live tokens. Anything under 4.5 is flagged.">
        <Sheet padding="sm">
          <div className="dev-contrast">
            {PAIRS.map(([f, b, min], i) => (
              <div key={f + b} className="dev-contrast__row" style={{ background: `var(--${b})`, color: `var(--${f})` }}>
                <span>{f} on {b}</span>
                <strong data-bad={ratios[i] < min || undefined}>{ratios[i] ? ratios[i].toFixed(2) : "..."}</strong>
              </div>
            ))}
          </div>
        </Sheet>
      </Section>

      <Section title="Type">
        <Sheet>
          <Stack gap={3}>
            {SIZES.map((n) => (
              <Row key={n} gap={4} align="baseline">
                <Text size="xs" color={3} mono style={{ width: "9rem", flex: "none" }}>{n} {values[n]}</Text>
                <span style={{ fontSize: `var(--${n})`, fontWeight: 600 }}>Sites should load.</span>
              </Row>
            ))}
            <Row gap={4} align="baseline">
              <Text size="xs" color={3} mono style={{ width: "9rem", flex: "none" }}>mono</Text>
              <Text mono>--lua-desync=fake:blob=fake_default_tls:badsum</Text>
            </Row>
          </Stack>
        </Sheet>
      </Section>

      <Section title="Space and shape">
        <Sheet>
          <Stack gap={5}>
            <Stack gap={2}>
              {SPACE.map((n) => (
                <Row key={n} gap={3}>
                  <Text size="xs" color={3} mono style={{ width: "9rem", flex: "none" }}>{n} {values[n]}</Text>
                  <div style={{ width: `var(--${n})`, height: "0.75rem", background: "var(--accent)" }} />
                </Row>
              ))}
            </Stack>
            <Row gap={5} wrap>
              {RADII.map((n) => (
                <Stack key={n} gap={1} align="center">
                  <div className="dev-radius" style={{ borderRadius: `var(--${n})` }} />
                  <Text size="xs" color={3} mono>{n} {values[n]}</Text>
                </Stack>
              ))}
            </Row>
          </Stack>
        </Sheet>
      </Section>

      <Section title="Elevation and motion" description="Only floating layers use a shadow.">
        <Sheet>
          <Stack gap={4}>
            <div className="dev-pop">--sh-pop</div>
            <Row gap={3} wrap>
              <Button variant="soft" tone="neutral" size="sm" icon="play" onClick={() => setFired((n) => n + 1)}>
                Play durations
              </Button>
            </Row>
            {DURATIONS.map((n) => (
              <Row key={n} gap={3}>
                <Text size="xs" color={3} mono style={{ width: "9rem", flex: "none" }}>{n} {values[n]}</Text>
                <div className="dev-track">
                  <div key={fired} className="dev-runner" data-run={fired > 0 || undefined} style={{ transitionDuration: `var(--${n})` }} />
                </div>
              </Row>
            ))}
          </Stack>
        </Sheet>
      </Section>
    </Stack>
  );
}
