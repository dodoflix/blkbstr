import { useEffect, useState } from "react";
import * as api from "../api";

export const SITES_KEY = "blkbstr.sites";

/** What the user changed about the built-in list: categories switched off, sites added. */
export type SitePrefs = { off: string[]; extra: string[] };

export const NO_PREFS: SitePrefs = { off: [], extra: [] };

export function readSitePrefs(): SitePrefs {
  try {
    const raw = JSON.parse(localStorage.getItem(SITES_KEY) ?? "null") as Partial<SitePrefs> | null;
    return { off: raw?.off ?? [], extra: raw?.extra ?? [] };
  } catch {
    return NO_PREFS;
  }
}

let categories: Promise<api.SiteCategory[]> | undefined;
let known: api.SiteCategory[] | undefined;
/** Fixed for the life of the build, so fetched once. */
export const siteCategories = () => (categories ??= api.siteCategories().then((c) => (known = c)));

/** Undefined only the first time, before the answer; after that a screen opens with it already in hand,
 *  so a list never draws flat and then regroups. */
export function useSiteCategories(): api.SiteCategory[] | undefined {
  const [value, setValue] = useState(known);
  useEffect(() => {
    if (!known) void siteCategories().then(setValue);
  }, []);
  return value;
}

/** What a pasted address becomes: `https://Example.com/page` is `example.com`. */
export const normalizeSite = (text: string) =>
  text
    .trim()
    .toLowerCase()
    .replace(/^[a-z]+:\/\//, "")
    .replace(/[/:?#].*$/, "");

/** The same rule the check applies, so a typo is refused here rather than reported as blocked. */
export const validSite = (host: string) => /^[a-z0-9.-]{1,253}$/.test(host) && host.includes(".");

/** The sites the next check should use: enabled categories plus the user's own. */
export async function selectedHosts(): Promise<string[]> {
  const { off, extra } = readSitePrefs();
  const builtIn = (await siteCategories()).filter((c) => !off.includes(c.id)).flatMap((c) => c.hosts);
  return [...new Set([...builtIn, ...extra])];
}
