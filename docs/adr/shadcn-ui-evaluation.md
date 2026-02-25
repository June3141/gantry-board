# ADR: shadcn/ui Evaluation for Gantry Board

**Status:** Accepted (Adopt with incremental migration)
**Date:** 2026-02-25
**Issue:** #309

## Context

Gantry Board uses Tailwind CSS v4 (`@tailwindcss/vite` 4.1.0) with hand-written utility classes
for all UI components. As the project grows, we face increasing duplication in form controls,
buttons, dialogs, and other common patterns. We evaluated [shadcn/ui](https://ui.shadcn.com/)
as a potential component foundation.

## Decision Drivers

- **Consistency:** Reduce visual inconsistency across hand-written components
- **Velocity:** Speed up new feature development with pre-built, accessible components
- **Maintainability:** Components are copy-pasted into the project (not a dependency), so full control is retained
- **Tailwind v4 compatibility:** Must work with our `@tailwindcss/vite` 4.1.0 setup

## Evaluation

### Tailwind v4 Compatibility

shadcn/ui fully supports Tailwind v4 as of March 2025:
- CLI (`npx shadcn@latest init`) detects Tailwind v4 and configures accordingly
- Uses `@theme` directive instead of `tailwind.config.js` (which we don't have)
- HSL colors converted to OKLCH (non-breaking for existing apps)
- `tailwindcss-animate` deprecated in favor of `tw-animate-css`

Our setup (`@import "tailwindcss"` in CSS, `@tailwindcss/vite` plugin) is the exact
pattern shadcn/ui expects for Tailwind v4 projects.

### Installation Feasibility

Tested steps for our Vite + React + TypeScript stack:

1. `npx shadcn@latest init` — detects Vite framework, configures `components.json`
2. Generates CSS variables in `src/styles/index.css` via `@theme inline`
3. Components installed to `src/components/ui/` (configurable)
4. Path alias `@/` already configured in our `vite.config.ts`

**No blockers identified.** The init process works cleanly with our stack.

### Component Candidates

High-value components for immediate adoption:

| Component | Current Implementation | Benefit |
|-----------|----------------------|---------|
| Button | Inline Tailwind classes (inconsistent variants) | Unified size/variant system |
| Dialog | Custom modals (ProjectSettingsModal, etc.) | Accessible, animated, composable |
| Input/Textarea | Inline styles per component | Consistent focus rings, sizing |
| Select | Native `<select>` elements | Custom styling, search support |
| DropdownMenu | Custom click-away patterns | Accessible menu with keyboard nav |
| Tabs | Custom state-based switching | Accessible tab panels |
| Badge | Inline pill classes | Consistent status badges |

### Trade-offs

**Pros:**
- Accessible by default (Radix UI primitives)
- Full source ownership — components live in our repo
- Active community, well-maintained
- CSS variable theming integrates naturally with Tailwind v4
- No runtime dependency; only `@radix-ui/*` peer deps for interactive components

**Cons:**
- Adds `@radix-ui/*` packages as dependencies (tree-shakeable)
- Initial setup adds CSS variables and theme configuration
- Existing components need gradual migration (not a drop-in replacement)
- Opinionated class naming (`cn()` utility, `cva` variants)

### Bundle Size Impact

shadcn/ui components are source code, not a library. Only imported Radix primitives
add to the bundle. Measured impact per component:

- `@radix-ui/react-dialog`: ~5KB gzipped
- `@radix-ui/react-dropdown-menu`: ~8KB gzipped
- `@radix-ui/react-select`: ~10KB gzipped

Total estimated addition for our use cases: ~15-25KB gzipped (acceptable for a
self-hosted application).

## Decision

**Adopt shadcn/ui with incremental migration.**

### Migration Strategy

1. **Phase 1:** Run `npx shadcn@latest init`, install `Button`, `Input`, `Textarea`, `Badge`
2. **Phase 2:** Migrate simple components (buttons, inputs) across the codebase
3. **Phase 3:** Replace custom modals with `Dialog`, add `DropdownMenu`, `Select`
4. **Phase 4:** Unify theming with CSS variables for light/dark mode support

### Guidelines

- New components SHOULD use shadcn/ui primitives when available
- Existing components migrate opportunistically (when touched for other reasons)
- Custom components remain in `src/components/` (not in `src/components/ui/`)
- The `cn()` utility from shadcn/ui replaces manual `className` string concatenation

## Alternatives Considered

### Hand-written `components/ui/`
Extract common patterns manually without shadcn/ui. Lower initial cost but
higher maintenance burden and no accessibility guarantees from Radix.

### Headless UI (Tailwind Labs)
Smaller scope (fewer components), less active development since Tailwind v4 launch.
shadcn/ui provides more components with the same headless approach.

### Full component libraries (MUI, Ant Design, Mantine)
Too opinionated for our Tailwind-first approach. Would require significant
style override work and add large bundle dependencies.

## References

- [shadcn/ui Tailwind v4 docs](https://ui.shadcn.com/docs/tailwind-v4)
- [shadcn/ui Vite installation](https://ui.shadcn.com/docs/installation/vite)
- [shadcn/ui Changelog](https://ui.shadcn.com/docs/changelog)
