import { useState } from "react";
import { Button, Checkbox, Dialog, Field, Stack, TagInput, Text, useLocalStorage } from "../ui";
import { NO_PREFS, normalizeSite, readSitePrefs, SITES_KEY, useSiteCategories, validSite, type SitePrefs } from "./sites";

/** Which kinds of site the check and the setup search use, and any sites of your own. Saved on
 *  this computer; the next check picks it up. */
function SiteChoices() {
  const categories = useSiteCategories() ?? [];
  const [prefs, setPrefs] = useLocalStorage<SitePrefs>(SITES_KEY, NO_PREFS);
  const off = prefs.off ?? [];

  return (
    <Stack gap={4}>
      <Stack gap={2}>
        {categories.map((c) => (
          <Checkbox
            key={c.id}
            label={c.label}
            description={c.hosts.join(", ")}
            checked={!off.includes(c.id)}
            onCheckedChange={(on) => setPrefs({ ...prefs, off: on ? off.filter((id) => id !== c.id) : [...off, c.id] })}
          />
        ))}
      </Stack>
      <Field label="Your own sites" hint="A strategy is only proven on the sites it was tried against, so add the ones you need.">
        <TagInput
          value={prefs.extra ?? []}
          placeholder="example.com, then Enter"
          validate={(tag) => (validSite(normalizeSite(tag)) ? undefined : "Enter a site name like example.com")}
          onChange={(next) => setPrefs({ ...prefs, extra: [...new Set(next.map(normalizeSite))] })}
        />
      </Field>
      {off.length === categories.length && categories.length > 0 && (prefs.extra ?? []).length === 0 && (
        <Text size="sm" color="danger">
          Nothing is selected, so the check has no sites to test.
        </Text>
      )}
    </Stack>
  );
}

/** A button and the dialog it opens. A dialog rather than a section that unfolds, because unfolding
 *  pushes everything under it down and the page shifts. */
export function ChooseSites({ onChanged }: { onChanged?: () => void }) {
  const [open, setOpen] = useState(false);
  const [before, setBefore] = useState("");
  return (
    <>
      <Button
        variant="soft"
        tone="neutral"
        size="sm"
        icon="list"
        onClick={() => {
          setBefore(JSON.stringify(readSitePrefs()));
          setOpen(true);
        }}
      >
        Choose sites
      </Button>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next && JSON.stringify(readSitePrefs()) !== before) onChanged?.();
        }}
        title="Sites to check"
        description="Setup and the site check use these. Check again afterwards to see the change."
        footer={<Button onClick={() => setOpen(false)}>Done</Button>}
        size="lg"
      >
        <SiteChoices />
      </Dialog>
    </>
  );
}
