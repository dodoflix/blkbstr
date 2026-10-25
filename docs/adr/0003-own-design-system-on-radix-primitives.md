# 3. Our own design system on Radix Primitives

Status: accepted

## Context

The UI needs one shape language, flat surfaces, light and dark themes and a first-run preferences
flow. A packaged component library (Radix Themes, or shadcn/ui over Tailwind) fights each of those:
every visual change becomes an override of a stylesheet we do not own.

## Decision

`src/ui` is the design system: tokens in CSS custom properties, components in TypeScript, behaviour
from the `radix-ui` primitives. Features in `src/features` compose it and never style from scratch.
See [../design-system.md](../design-system.md).

## Why

Accessibility still comes from the primitives (focus, ARIA, keyboard in dialogs, menus, tabs,
selects). The look comes from tokens we own, so a theme or motion change is one attribute on
`<html>`, and there is no component library stylesheet in the bundle.

## Costs

We maintain the components. The Developer page (dev builds only) renders every one, which is the
regression surface.

## Alternatives

**Radix Themes** - least code, but the look is set by its props and tokens, and it adds a large
stylesheet.

**shadcn/ui + Tailwind** - same primitives, but a utility-class build in the loop for no gain over
tokens.
