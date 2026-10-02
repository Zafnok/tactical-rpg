---
id: "0436"
title: "Units on the battle map drawn as the bought map sprites"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0433", "0110", "0039"]
nick_input: sign-off
completed:
---

# 0436 — Units on the battle map as the bought map sprites

## Context

Nick decided on 2026-10-02 (ticket 0038, `docs/design/look-and-feel.md`,
*Battle map: bought tiles and unit sprites*) that the battle map is drawn
with the bought Tiny Tales art. This ticket does the units: each unit is
its **map sprite** instead of two letters. Terrain is 0437; until that
lands, sprites stand on the glyph terrain (render B of the spike, which
Nick also liked).

ADR-0038: units are painted by a map skin from the `MapScene` (0432), and
pictures on the map come from a tileset file through the sprite skin
(0433). 0433's skin reads every unit picture from one grid in one image
and uses a generated test tileset. The bought sprites are different:

- **One file per character or class**, a 48×80 sheet: 3 columns (walking
  frames) × 4 rows (facing down, left, right, up) of **16×20** frames. The
  standing, front-facing frame is column 1 (the middle), row 0.
- Hundreds of them: the 16 heroes, about 210 battler classes, and map
  sprite packs with no battle picture (townsfolk, knights, nobles, 1100
  generated people). Only the ones a chapter uses are imported.
- They are bought, so they live in `assets-private/` (0110), and a clone
  without them must still build, run and pass every gate.

The bought files are sorted on Nick's machine in
`D:\tactical-rpg\Tiny Tales Bundle Assets\characters\` (`heroes/<Name>/
map_sprite.png`, `battler-classes/<pack>/<Name>/map_sprite.png`,
`map-sprites-only/<pack>/<Name>.png`); `INDEX.md` there lists them. 0110
decides how they reach `assets-private/`.

## Nick input

**Answer first:** ticket 0039, question 1 (how a sprite unit shows its
side and that it has acted), question 4 (standing or walking) and question
6 (the effect mark). Build what it records. If 0039 put a question off,
use the spike's look for that part and mark it *Claude's starting rule* in
the PR: a 1-pixel outline in the side's colour, an acted unit drawn grey
and darker, a standing front-facing frame, 0433's corner mark for an
effect.

**Sign-off:** a rendered Quick Battle frame with the bought sprites, sent
to Nick (never committed), and the list of which sprite each Chapter 1
class and character uses.

## Scope

**In:**
- Unit pictures in a tileset file may come from **separate image files**,
  one per character or class, as well as from 0433's grid.
- A skin that paints **terrain as glyphs and units as sprites** (so this
  ticket doesn't wait for 0437).
- The marks 0039 decided (side, acted, effect), the HP bar and the
  fading-out of a falling unit, on sprites.
- A private tileset file naming the bought map sprite of every class and
  named character in the Quick Battle and Chapter 1, and an importer that
  copies those files into `assets-private/`.
- A public placeholder so the public build still runs: without
  `assets-private/` the game keeps the glyph skin.
- The game uses the sprite-unit skin by default **when the private tileset
  is present**.

**Out (do not do):**
- Terrain tiles (0437), lighting (0438), zoom (0439).
- Walking animation along a path, unless 0039's question 4 asks for it;
  if it does and it doesn't fit in this PR, write a follow-up ticket.
- An Options entry for the look (0039 question 5; 0805).
- Drawing or editing sprites. Recolours are allowed (`look-and-feel.md`)
  but not needed here.
- The combat scene (0413) and portraits (0711).
- Committing any bought file, or a picture made from one.

## Implementation steps

1. **ADR** (`write-adr`), extending ADR-0038 section 3 and 0433's format:
   how a tileset file names per-unit image files, how the game picks its
   map skin (private tileset present → that skin; otherwise the glyph
   skin), and that a skin may mix glyph terrain with sprite units.
2. **Format** (`crates/content/src/tileset.rs`, `assets/tilesets/README.md`):
   in `units`, an entry is either 0433's `(column, row)` in the tileset's
   own image or `(image: "units/fighter_male.png", frame: (1, 0))` with
   `unit_px: (16, 20)` giving the frame size. Validate: the image is in
   `Content::images`, the frame lies inside it, every named class and
   character exists. A tileset may have `terrain: None` (meaning "paint
   terrain with the glyph skin"); 0433's rule "every terrain id has a
   tile" then doesn't apply.
3. **Skin** (`crates/ui/src/map_view/`): where a tileset has no terrain,
   paint terrain, ranges, cursor and path with `GlyphSkin` and units with
   `SpriteSkin`'s unit code. Share, don't copy. A unit's tile keeps its
   terrain background and loses the terrain's glyphs under the sprite (as
   the glyph skin does under initials). Sprites are drawn top row first so
   a unit's head overlaps the tile above it and not the reverse.
4. **Marks** per 0039. An outline needs the sprite's silhouette: draw the
   same frame four times, offset by one pixel up, down, left and right,
   in the side's colour, under the sprite. That needs a **solid-colour
   draw of a sprite** (every opaque pixel in one colour): add it to
   `Sprite` (the item, `app`'s renderer, the snapshot line, 0232's PNG
   renderer if done) and note it in the ADR. 0413 needs the same thing for
   its hit flash: whichever ticket lands first adds it, the other reuses
   it. Acted: greyed and darker, by the same means or by opacity, as 0039
   says. Never per-pixel rectangles (ADR-0038).
5. **Unit lookup**: character id, then class id, then `fallback` (0433).
   Write the Chapter 1 and Quick Battle table in the private tileset file
   from `assets/data/characters.ron` and `classes.ron`. Starting picks,
   for Nick's sign-off (from the spike):
   - lead: Heroes 1 Male Fighter / Female Fighter (by the lead's sex);
   - Mage (Rue): Heroes 1 Witch; Archer: Heroes 1 Archer;
   - Guard: *Faith and Evil* Church Knight; Cleric: Church Cleric;
   - Rider: nothing mounted exists in the bundle; a Human Knights sprite
     on foot until 0040 finds mounted art;
   - Brigand, Raider, enemy Archer: Human NPC Advanced `Warrior_M*`,
     `Fighter_M*`, `Rogue_M*` (human bandits, where the combat picture is
     still the orc stand-in);
   - Fire and Frost Elemental: *Elemental Forces*.
   A unit's map sprite and its combat picture (0413) should be the same
   character where both exist.
6. **Importer** (`cargo xtask`, next to 0711's `portrait-import`): copy the
   named map sprite sheets into `assets-private/units/` under stable
   names. Clear error when the source folder is missing.
7. **Default skin**: at start-up `Ctx.map_skin` is the private tileset's
   skin when `Content::tilesets` has it, else `GlyphSkin`. The debug-menu
   switch from 0433 keeps working and lists it.
8. Render the Quick Battle with `cargo xtask frame-png` (0232 if done;
   otherwise the web build's debug menu) with the bought sprites, **look
   at it**, and send it to Nick. Don't commit it.
9. Update `crates/ui/README.md`, `assets/tilesets/README.md` and
   `look-and-feel.md` (the class → sprite table, once Nick signs off).

## Acceptance criteria

- [ ] Content validation: each new error has a test with its message (a
      unit image that doesn't exist, a frame outside its image, an unknown
      class or character).
- [ ] A clone **without** `assets-private/` builds, runs with the glyph
      skin and passes every gate; every existing glyph snapshot is
      unchanged.
- [ ] With the private tileset present, the Quick Battle shows every unit
      as a sprite on glyph terrain, with the side mark, the acted look and
      the HP bar (snapshot on a **public fixture** tileset with per-file
      unit images, made by `cargo xtask test-tileset`; never a bought
      file).
- [ ] 0433's `the_skin_never_changes_the_game` and
      `every_scene_feature_is_painted` also run under the mixed skin.
- [ ] `git status` shows no bought file; the PR has no picture made from
      one.
- [ ] Nick was sent the rendered frame and the class → sprite list.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the tileset format's new entries and errors; unit lookup order;
  the frame rectangle for `(1, 0)` in a 48×80 sheet; draw order (top row
  first); the outline's four offsets.
- Property: for any unit frame size up to the tile size plus 16 pixels,
  the sprite's `dest` is centred on its tile and bottom-aligned.
- Snapshot / integration: Quick Battle under the mixed skin on the public
  fixture; behaviour tests read the `MapScene` (ADR-0038), not sprites.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
