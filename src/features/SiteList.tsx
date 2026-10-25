import type { Report, SiteResult } from "../api";
import { Accordion, Badge, RowList, Skeleton, Stack, StateMark, Text } from "../ui";
import { VERDICTS } from "./format";
import { usePrefs } from "./prefs";
import { useSiteCategories } from "./sites";

function row(site: SiteResult, advanced: boolean) {
  const v = VERDICTS[site.verdict];
  return {
    id: site.host,
    leading: <StateMark state={v.state} size={18} label={v.label} />,
    title: site.host,
    subtitle: advanced ? site.detail : undefined,
    trailing: (
      <>
        <span>{v.label}</span>
        {advanced && (
          <Text mono size="sm" color={3}>
            {site.elapsed_ms}ms
          </Text>
        )}
      </>
    ),
  };
}

/** One row per site, grouped by category and collapsed until opened: shape-coded verdict, plain-language label, timing and
 *  detail when advanced. Sites the user added come last, under their own heading. */
export function SiteList({ report }: { report: Report }) {
  const { advanced } = usePrefs();
  const categories = useSiteCategories();
  if (!categories) return <SiteListPlaceholder />;

  const groups = categories
    .map((c) => ({ id: c.id, label: c.label, sites: report.sites.filter((s) => c.hosts.includes(s.host)) }))
    .filter((g) => g.sites.length > 0);
  const known = new Set(groups.flatMap((g) => g.sites.map((s) => s.host)));
  const added = report.sites.filter((s) => !known.has(s.host));
  if (added.length > 0) groups.push({ id: "added", label: "Added by you", sites: added });
  if (advanced) groups.push({ id: "control", label: "Control", sites: [report.control] });

  // Before the categories arrive, or with nothing to group, a flat list beats an empty one.
  if (groups.length <= 1) {
    return <RowList label="Site check results" items={[...report.sites, ...(advanced ? [report.control] : [])].map((s) => row(s, advanced))} />;
  }
  return (
    <Accordion
      items={groups.map((g) => {
        const blocked = g.sites.filter((s) => s.verdict !== "fine").length;
        return {
          value: g.id,
          title: g.label,
          meta:
            g.id === "control" ? undefined : (
              <Badge tone={blocked === 0 ? "ok" : "warn"}>{blocked === 0 ? "all load" : `${blocked} of ${g.sites.length} blocked`}</Badge>
            ),
          content: <RowList label={`${g.label} sites`} items={g.sites.map((s) => row(s, advanced))} />,
        };
      })}
    />
  );
}

const PLACEHOLDER_ROWS = 6;

/** The collapsed list's footprint, for while the sites or the answers are on their way. */
export function SiteListPlaceholder() {
  return (
    <Stack gap={0} aria-hidden>
      {Array.from({ length: PLACEHOLDER_ROWS }, (_, i) => (
        <div className="site-skeleton" key={i}>
          <Skeleton width={`${24 + ((i * 13) % 26)}%`} height={14} />
        </div>
      ))}
    </Stack>
  );
}
