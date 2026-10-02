---
id: "0439"
title: "A key and a button toggle the battle map's zoom (1× / 2×), and the game remembers it"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0433"]
nick_input: decision
completed:
---

# 0439 — Map zoom toggle (1× / 2×), remembered

## Context

Nick, 2026-10-02 (ticket 0038), choosing between the bought tiles at
their own size (spike render C: a 16-pixel tile, about 34×28 tiles on
screen) and at double size (render C2: a 32-pixel tile, about 17×14 tiles,
close to Fire Emblem on the GBA): "A key/button to toggle zoom (1x / 2x)
with memory (i.e. going to next battle, keep the setting)".

Rule: `docs/design/look-and-feel.md`, *Battle map: bought tiles and unit
sprites*, *Zoom*.

ADR-0038: "Tile size belongs to the skin. The camera and viewport ask the
skin how many tiles fit." 0432 and 0433 build that; this ticket makes the
sprite skin's tile size switchable while a battle is running.

## Nick input

**Decision, asked at the start of this ticket** (`ask-nick`, before any
code): the default key in both layouts, the controller button, and
whether the action is optional. Default keys are Nick's (`keyboard-input`
skill, rule 4); don't pick them yourself. Asked once on 2026-10-02, Nick
put it off to this ticket: "I think we can ask this as we build the zoom
feature in another ticket? I will decide it later".

What was offered then, to offer again with anything else that is free in
`docs/design/controls.md`: `Q` in the right-handed layout and `P` in the
left-handed one (finger mirrors, beside the other keys); the right
trigger (`RT` / `R2` / `ZR`) on a controller (the left trigger is
Rewind); optional, like Unit info and Rewind (may be left without a key).
Record his answer in `controls.md`: the default-keys table, the
controller table and the required/optional lists.

**Sign-off:** Nick tries it in the Pages build after merge (the toggle on
the test tileset from the debug menu if the bought-art skin isn't in yet).

## Scope

**In:**
- A new input action, Zoom map, per the `keyboard-input` skill.
- The sprite skin draws at 1× or 2×: tiles, unit sprites, HP bars, marks,
  cursor and path all scale together.
- The camera keeps the cursor on screen when the zoom changes, and
  scrolls as it does today.
- The choice is saved with the player's settings and used by the next
  battle and the next time the game starts.

**Out (do not do):**
- Other zoom levels, smooth zooming, a mouse wheel.
- Zooming the glyph skin: its tile is two font cells. With the glyph skin
  the action does nothing and the help bar doesn't list it. (0228's
  cinematic camera zooms a glyph scene for cutscenes; it isn't reused
  here.)
- An Options-menu row, unless 0805 is already done: then add "Map zoom:
  1× / 2×" there; if not, add a line to 0805 to do it.
- Zooming the side panel, menus or text.

## Implementation steps

1. **Action** (`keyboard-input` skill, *Adding an action*): `Action::ZoomMap`
   in `crates/content/src/keymap.rs`, in `Action::ALL`, required or
   optional as Nick answered; defaults in `assets/data/keymap.ron` for
   both layouts and the `pad` table exactly as recorded in `controls.md`;
   a row in the Key bindings screen (`ROWS`).
2. **Skin**: `SpriteSkin` gets a `zoom: u8` (1 or 2). `tile_px` and
   `unit_px` are multiplied by it; every `dest` is in whole console
   pixels, so art pixels stay square. `view_tiles(area)` already follows
   `tile_px` (0433). HP bar thickness, the side mark, the cursor's corner
   arms and the path's width scale by the zoom too.
3. **Setting**: `map_zoom` in the player's `Settings` (the same storage as
   `cursor_style`; if 0805's `Settings` struct isn't there yet, keep it in
   `Ctx` and persist it under its own storage key, and add a line to 0805
   to move it). Default 1×. An unreadable or out-of-range saved value
   falls back to 1× without an error.
4. **Battle screen**: on `Action::ZoomMap`, flip the setting, save it,
   rebuild the skin's zoom, and re-centre the camera so the cursor's tile
   stays visible (clamped to the map as scrolling is today). Works while
   browsing, with a unit selected, while targeting, and in the enemy
   phase; does nothing in menus that cover the map. Not a `Command`: the
   zoom is not game state (ADR-0004, ADR-0038), so rewind, bots and play
   records never see it.
5. **Help**: name the action in the map menu's or help bar's key list via
   `ctx.help_keys()` (rule 2), only when the current skin can zoom. Check
   `the_longest_help_bar_fits_on_every_pad` still passes.
6. Render Quick Battle on the test tileset at both zooms with
   `cargo xtask frame-png` (0232 if done) and **look at them**: nothing
   cut at the view's edge, a menu opened beside a unit still sits beside
   it at 2×.

## Acceptance criteria

- [ ] Nick's key, button and required-or-optional answer is in
      `controls.md` with his words, and `keymap.ron` matches it (the
      keymap-matches-design test).
- [ ] Pressing the Zoom map key on the sprite skin switches between 1×
      and 2×; the cursor's tile is inside the view after each switch (a
      scene assertion, for cursor positions at each map corner).
- [ ] The zoom survives leaving the battle and starting another, and a
      restart (a storage test).
- [ ] With the glyph skin the action changes nothing, and the help text
      doesn't list it.
- [ ] `the_skin_never_changes_the_game` (0433) passes with the script run
      at 2×: same `BattleState`, same cursor tiles, same sounds.
- [ ] Rebinding the action works, and it shows `! not mapped` when
      unbound (if optional).
- [ ] Snapshots of Quick Battle on the test tileset at 2×.
- [ ] `cargo xtask check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `view_tiles` and every `dest` at both zooms; the camera
  re-centring; the setting's load, save and fallback.
- Property: at either zoom, every painted rectangle lies inside the map
  area and tiles don't overlap.
- Snapshot / integration: as listed, plus one Harness test using the
  default key and one with the action rebound.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
