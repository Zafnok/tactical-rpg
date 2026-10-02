---
id: "1006"
title: Explore custom 16×16 class icons for units on the map
type: design-decision
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: blocked
blocked_by: ["0401", "0021", "0110", "0711", "0433", "0039"]
nick_input: decision
completed:
---

# 1006 — Explore custom class icons

## Context

In ticket 0011 Nick found the font's symbols (`†`, `»`, `}` …) not
expressive enough for units: "other terminal type games are more expressive of
units / characters than this". He settled on name initials plus an HP bar
(`docs/design/look-and-feel.md`). Custom-drawn icons, like Caves of Qud's
tiles, were offered as a later experiment: one small single-colour picture per
class (a helmeted head, a horse head, a drawn bow…), 16×16 px, tinted by
faction like any glyph.

**Changed 2026-09-30:** Nick doesn't want Claude-drawn character art (0021),
and the combat scene uses bought sprites (0413). The packs 0021 picked
(Mega Tiles' Tiny Tales) include **16×20 map sprites** for every hero and
still-battler class: offer those as well as Claude-drawn one-colour icons,
and say honestly that Claude-drawn icons may clash in the same way the
portraits did. A 16×20 sprite is taller than a 16×16 tile; show how that
looks. Showing the bought map sprites on the real battle screen needs the
bought files (0110) and the PNG drawing (0711): both were added to
`blocked_by` on 2026-10-01.

**Changed 2026-10-01 (ADR-0038):** units on the map are painted by a map
skin (0432), and pictures on the map come from a tileset file through the
sprite skin (0433, added to `blocked_by`). So the versions shown to Nick
are skins or tileset files, not new drawing code in the battle screen:
initials (the glyph skin), bought map sprites over glyph terrain, and
Claude-drawn one-colour icons. How a sprite unit shows its side, that it
has acted, its HP and an active effect is part of this decision: 0433's
versions of those are placeholders.

**Changed 2026-10-02 (ticket 0038): the main question is answered.** Nick
bought the whole Mega Tiles bundle, saw the bought 16×20 map sprites on
the battle screen (over glyph terrain and over the bought tilesets) and
chose them: "I think we can go ahead and move forward with replacing our
tile rendering and battler rendering with these bought sprites without
regret". Units become bought map sprites in ticket 0436 and terrain bought
tiles in 0437; a sprite unit shows its side by a coloured outline and
"acted" by going grey (decided the same day, built in 0436); the effect
mark is ticket 0039. **What is left of this ticket** is only the Claude-drawn
one-colour class icons, which Nick didn't ask for and which may clash as
the Claude-drawn portraits did. Don't start it unless Nick asks for icons
(for example as a mark on the glyph look, if 0039 keeps that look
selectable). `status: blocked` until then; Nick unblocks it by asking.

## Nick input

**Decision:** Nick compares icons (drawn, and the bought pack's map sprites
if it has them) with initials on the real battle screen and picks one (or a mix, e.g. icon + HP bar, initials in the side panel).

## Scope

**In:** drawing icons for the Chapter 1 classes; rendering the battle screen
both ways for Nick; if chosen, making it a setting or the default.

**Out (do not do):** changing the font; icons for classes not yet in the game.

## Implementation steps

1. Draw 16×16 one-colour icons as a PNG in a tileset file (0433's format),
   tinted by faction if the skin needs a tint (add it to `Sprite` as 0413
   describes). For the bought map sprites, a tileset file in
   `assets-private/` pointing at the bought sheet. If units should be
   pictures while terrain stays glyphs, add that as a skin that paints
   terrain with the glyph skin and units with the sprite skin.
2. Render the Quick Battle screen each way (with HP bars) with
   `cargo xtask frame-png` (0232, if done) and ask Nick with `ask-nick`.
3. Record the answer in `look-and-feel.md`; implement if chosen.

## Acceptance criteria

- [ ] Nick saw both versions and his choice is recorded.
- [ ] If adopted: implemented with snapshots; all gates pass.

## Tests required

- Snapshot: battle screen with icons (if adopted).

## Completion notes

