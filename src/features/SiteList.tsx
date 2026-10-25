import type { Report, SiteResult } from "../api";
import { RowList, StateMark, Text } from "../ui";
import { VERDICTS } from "./format";
import { usePrefs } from "./prefs";

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

/** One row per site: shape-coded verdict, plain-language label, timing and detail when advanced. */
export function SiteList({ report }: { report: Report }) {
  const { advanced } = usePrefs();
  const items = [...report.sites, ...(advanced ? [report.control] : [])].map((s) => row(s, advanced));
  return <RowList label="Site check results" items={items} />;
}
