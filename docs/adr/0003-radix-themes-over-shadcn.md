# 3. Radix Themes rather than shadcn/ui

Status: accepted

## Context

The original brainstorm named "React + shadcn + Radix UI, Tailwind as a last resort — prefer
component-provided customization first". Those two halves conflict: shadcn/ui is Tailwind classes
over Radix primitives, and there is no way to adopt it without adopting Tailwind.

## Decision

`@radix-ui/themes`. No Tailwind, no CSS framework.

## Why

It resolves the conflict in favour of the stated preference. Radix Themes is the same Radix
primitives with a design system already on top, customised through component props and theme
tokens rather than utility classes — which is what "component-provided customization first" asks
for.

Accessibility comes from the primitives underneath, which matters for the M4 accessibility pass:
keyboard navigation and correct ARIA in dialogs, tabs and menus are the parts nobody wants to
write by hand, and they are already correct here.

Light/dark is a single `appearance` prop, so following the OS theme is one `matchMedia` hook
rather than a colour-scheme implementation.

## Costs

The stylesheet is around 700 KB before gzip, which is heavy for what the app currently renders.
It is a local file in a desktop bundle rather than a page load, so the cost is bundle size, not
latency — revisit only if the bundle becomes a real constraint.

Less freedom for unusual layouts than utility classes give. For a settings-and-wizards app this is
mostly a benefit.

## Alternatives

**shadcn/ui + Tailwind** — the largest component ecosystem and full control, at the cost of the
stated preference and a Tailwind build in the loop.

**Radix Primitives alone** — maximum control, but every visual decision becomes ours, and the
design work would not obviously come out better.

## Revisiting

If a screen genuinely cannot be built with Themes, adding Radix Primitives directly alongside is
cheap — same underlying library. That is the escape hatch, not a rewrite.
