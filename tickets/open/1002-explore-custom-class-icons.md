---
id: "1002"
title: Explore custom 16×16 class icons for units on the map
type: design-decision
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0401"]
nick_input: decision
completed:
---

# 1002 — Explore custom class icons

## Context

In ticket 0011 Nick found the font's symbols (`†`, `»`, `}` …) not
expressive enough for units: "other terminal type games are more expressive of
units / characters than this". He settled on name initials plus an HP bar
(`docs/design/look-and-feel.md`). Custom-drawn icons, like Caves of Qud's
tiles, were offered as a later experiment: one small single-colour picture per
class (a helmeted head, a horse head, a drawn bow…), 16×16 px, tinted by
faction like any glyph.

## Nick input

**Decision:** Nick compares icons with initials on the real battle screen and
picks one (or a mix, e.g. icon + HP bar, initials in the side panel).

## Scope

**In:** drawing icons for the Chapter 1 classes; rendering the battle screen
both ways for Nick; if chosen, making it a setting or the default.

**Out (do not do):** changing the font; icons for classes not yet in the game.

## Implementation steps

1. Draw 16×16 one-colour icons (e.g. as 2-cell-wide custom glyphs appended
   to the font atlas in a private-use range, via `cargo xtask font-atlas`, or
   as pixel grids drawn with overlays). Pick the technique and record it in an
   ADR if it changes the atlas format.
2. Render the Quick Battle screen with initials vs icons (with HP bars) and
   ask Nick with `ask-nick`.
3. Record the answer in `look-and-feel.md`; implement if chosen.

## Acceptance criteria

- [ ] Nick saw both versions and his choice is recorded.
- [ ] If adopted: implemented with snapshots; all gates pass.

## Tests required

- Snapshot: battle screen with icons (if adopted).

## Completion notes

