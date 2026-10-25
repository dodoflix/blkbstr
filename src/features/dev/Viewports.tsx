import { useLayoutEffect, useRef, useState } from "react";
import { Field, Row, Select, Stack, Text } from "../../ui";

const SIZES = ["720x540", "859x600", "861x600", "980x700", "1280x800", "1440x900"];
const PAGES = ["home", "configs", "logs", "system"];

/** The live app in an iframe of an exact size, so media queries fire as they would in a window of
 *  that size. Scaled down to fit, never up. */
export function Viewports() {
  const [size, setSize] = useState("980x700");
  const [page, setPage] = useState("home");
  const [w, h] = size.split("x").map(Number);
  const box = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const fit = () => setScale(Math.min(1, el.clientWidth / w));
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [w]);
  const src = `${location.pathname}${location.search}#${page}`;
  return (
    <Stack gap={4}>
      <Row gap={3} wrap align="flex-end">
        <Field label="Window size">
          <Select value={size} onValueChange={setSize} options={SIZES.map((s) => ({ value: s, label: s }))} />
        </Field>
        <Field label="Page">
          <Select value={page} onValueChange={setPage} options={PAGES.map((p) => ({ value: p, label: p }))} />
        </Field>
      </Row>
      <Text size="sm" color={3}>
        Uses the live backend (or the mock, in a browser). Theme and motion follow the bar above.
      </Text>
      <div className="dev-viewport" ref={box} style={{ aspectRatio: `${w} / ${h}`, maxWidth: w }}>
        <iframe key={size + page} title={`App at ${size}`} src={src} className="dev-viewport__frame" style={{ width: w, height: h, transform: `scale(${scale})` }} />
      </div>
    </Stack>
  );
}
