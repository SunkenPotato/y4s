# Corner design system

Corner is a student-only guide to places near campus. Verified students review local businesses and campus facilities.

- Tokens: `design/tokens.css`. The frontend copies it to `web/src/styles/tokens.css` and treats it as the only source of truth.
- Reference artboards:
  - Round 3, the locked design: https://claude.ai/artifact/34Xvjva3Q6MGXDEXUVjDfe
  - Round 2, the A2 screens for feed, place page and phone: https://claude.ai/artifact/Y5JUPv8g2BuvMe8HmfpvRQ

## Thesis

Reviews are the product. The feed leads with a large pull-quote from a top review. Everything around it stays quiet: one typeface, one green accent, and grouping by spacing and soft filled tiles.

## Hard rules (the style guard enforces these)

1. **No gradients** of any kind (`linear-gradient`, `radial-gradient`, `conic-gradient`).
2. **No glow or diffuse shadows.** The only shadow allowed is a 1px hairline used as a tab-bar edge (`0 -1px 0`) or a 3px ring that separates map pins. Never a blur over 4px.
3. **No monospace.** Schibsted Grotesk is the only typeface, including for numbers. Use `font-variant-numeric: tabular-nums` where alignment matters.
4. **No divider lines or outlines for grouping.** Group with spacing (`--space-*`) and `--color-surface` tiles. Borders are allowed only on form fields for focus and error states (2px) and on the focus ring.
5. **No decorative micro-labels.** That means no eyebrows, no ALL-CAPS tracking labels, no 01/02/03 on content that isn't a sequence, and no emoji. Icons are inline stroke SVGs, 2px stroke, `currentColor`.
6. **The accent is for actions and trust signals only:** primary buttons, stars, the verified badge, deals and the selected tab. It is never used as decoration.
7. **Light and dark both follow the OS** (`prefers-color-scheme`). There is no in-app toggle.

## Type scale

| Style | Spec | Use |
| --- | --- | --- |
| Display | 800, clamp(36–52px), -0.035em | Page titles: "Near UCLA", place name |
| Title 1 | 800 34px, -0.035em | Phone screen titles, form titles |
| Title 2 | 700 26px, -0.02em | Section headings |
| Headline | 700 21px, -0.015em | Card titles |
| Body | 400 16px/1.5 | Default |
| Body large | 400 17px/1.55 | Review text |
| Callout | 500 15px | Segmented controls, chips |
| Caption | 400 14px | Meta lines, helper and error text (14px is the minimum size) |
| Tab label | 500–600 12px | Only under `TabBar` icons, following the platform tab-bar convention |

## Components (`web/src/components/`)

Pages are built only from these. If a page needs something new, add it here first.

| Component | Notes |
| --- | --- |
| `Button` | Variants: `primary` (accent), `secondary` (surface), `ghost` (accent-ink text), `danger` (danger-soft). Sizes: 48 (default), 44, 36 (pill, used inside rows). `loading` shows a spinner plus a verb ending in "…" ("Posting…") and sets `aria-busy`. Pressing scales to 0.97 over 160ms with ease-out. |
| `IconButton` | 44×44, `aria-label` required. |
| `Field` | Label above the input, helper text below. The error state uses a danger-soft fill, a 2px danger border, and the message replaces the helper text, linked with `aria-describedby`. |
| `Input`, `Textarea`, `Select`, `SearchField` | Surface fill, 12px radius, 48px tall, a 2px accent border on focus. `Textarea` has a character counter. |
| `Chip` / `FilterChips` | Pill shape. Selected chips are ink on bg (inverted). Uses `aria-pressed`. |
| `Segmented` | Surface track; the selected segment is a bg-colored pill. |
| `StarRating` | Display mode is a ★ string in accent with an `aria-label`. Input mode is a radiogroup of 44px stars. |
| `RatingPill` | "★ 4.6" on bg (on a surface tile) or on surface (on bg). |
| `Badge` | Tones: `verified` (accent-soft / accent-ink), `deal` (accent / on-accent), `neutral` (surface), `pending` (warn), `rejected` (danger). |
| `Alert` | Tones: success, warning, error. A soft fill, an icon and a bold first line. Errors use `role="alert"`. |
| `Toast` | Ink background. Its only action is Undo. Lasts 5s and pauses while hovered. |
| `PlaceTile` | Surface tile: meta line plus a rating pill, a Headline name, a quote, and a footer with the reviewer and the deal. |
| `FeaturedPlace` | Accent-soft hero holding the pull-quote. One per page. |
| `ReviewItem` | Avatar initials, name, verified badge, date, stars, body, Helpful and Report. Items are separated by a 40px gap, never a line. |
| `Avatar` | Initials on surface; 30, 36 or 44px. |
| `EmptyState` | 64px surface icon tile, a Title 2 line, one sentence and one primary action. |
| `Header` | Wordmark, search field, then the nav. Wraps on narrow screens. |
| `TabBar` | Mobile only (<720px). Tabs: Explore, Deals, Review, Profile. A 1px surface edge on top, selected tab in accent. |
| `Map` | Leaflet with OSM tiles, inside a surface tile with a 16px radius. Pins are accent squares with a 3px bg ring. |

Supporting components added during the build, under the same rules:

| Component | Notes |
| --- | --- |
| `Icon` | Shared stroke icon set |
| `Dialog` | Native `<dialog>`; becomes a bottom sheet on phones |
| `Checkbox`, `ChoiceList` | |
| `Tile` | Surface container |
| `RatingSummary` | |
| `StepList` | |
| `HoursList` | |

## Layout

- The content column is `max-width: var(--page-max)` with `padding-inline: var(--page-gutter)`.
- Main column plus side column: a flex-wrap row with `flex: 999 1 560px` for main and `flex: 1 1 300px` for the aside. The aside stacks below main on narrow screens.
- Below 720px the desktop nav moves into the `TabBar` and search stays at the top.

## Motion

- Every pressable element shrinks to 0.97 on press over 160ms `var(--ease-out)`.
- Hover effects are gated behind `@media (hover: hover) and (pointer: fine)`.
- Popovers and menus scale from 0.97 with a fade over 200ms, starting from the trigger's side.
- Sheets on mobile slide in over 300ms with `--ease-out`.
- No animation on keyboard-driven actions, list filtering or tab switches.
- With `prefers-reduced-motion`, remove transform animations and keep opacity and color transitions.

## Writing

- Buttons say what happens: "Post review", "Request access", "Verify". A button keeps its name through the flow: "Posting…" becomes "Review posted".
- Errors say what's wrong and how to fix it, without apologizing: "Use the email your school gave you, like name@ucla.edu."
- Use sentence case everywhere.
- Reviewers are shown as first name, last initial and school, never by email.
