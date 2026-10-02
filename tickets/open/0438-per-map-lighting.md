---
id: "0438"
title: "Per-map lighting: low-light maps are shaded, noon maps are not"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0437"]
nick_input: sign-off
completed:
---

# 0438 — Per-map lighting

## Context

Nick, 2026-10-02 (ticket 0038), choosing between the bright bought tiles
(spike render C) and the same tiles darkened toward our earthy palette
(render C3): "Per-map lighting effects (i.e. if it takes place during what
would be a low light time, it can be shaded like with C3. Or if it's noon,
it can be like C/C2."

Rule, in `docs/design/look-and-feel.md` (*Battle map: bought tiles and
unit sprites*): lighting is set per map; noon shows the tiles as bought;
low light shows them darkened; unit sprites keep their own colours in
every light (*Claude's starting rule*, from how C3 was drawn).

How C3 was made (the look Nick approved): each terrain pixel was moved 35%
of the way to its own grey, then its red, green and blue were multiplied
by 0.62, 0.56 and 0.46. Units, the cursor, the HP bars and the UI were not
touched.

0437 draws the terrain with sprite items and gives a map file a `look`
header. A sprite item has no tint yet (ADR-0038 section 2; 0413 and 0436
add other sprite effects).

## Nick input

**Sign-off:** one map rendered in each light, side by side, with bought
tiles and units (never committed), and the list of which light each
existing map got. He may ask for other lights or strengths: the values are
data.

## Scope

**In:**
- A small set of named lights as data, each a recipe for shading terrain.
- A map's light, named in its `look` header; default noon.
- The sprite skin shades terrain tiles by the map's light. Range tints,
  cursor, path, HP bars and unit sprites are drawn as before, on top.
- The combat scene's background (0413), if it uses the bought battle
  backgrounds: they come in day, dusk and night versions. Add a line to
  0413 (or to its follow-up if 0413 is done) that the background's version
  follows the map's light. Don't build it here.

**Out (do not do):**
- Lights that change during a battle, light sources, shadows, weather.
- Shading the glyph skin (it is already dark) or the UI.
- Shading unit sprites.
- Colour themes (0806): a theme never recolours a picture.

## Implementation steps

1. **Data** `assets/data/lighting.ron`: `{ "noon": (grey: 0.0, tint: (1.0,
   1.0, 1.0)), "dusk": (grey: 0.35, tint: (0.62, 0.56, 0.46)), … }`.
   Start with **noon** (as bought), **dusk** (C3's values above) and
   **night** (darker and bluer than dusk: *Claude's starting values*, e.g.
   grey 0.45, tint (0.40, 0.44, 0.58)). All *tunable*. Loader and validator
   in `trpg-content` (values in `0.0..=1.0`; `noon` must exist).
2. **Map look**: `look: (tiles: "outdoor", light: "dusk")` in the `.map`
   header (0437's `MapLook`); unknown light name is a validation error
   with file and line. A battle file (`assets/battles/*.ron`) may override
   its map's light with an optional `light:` so one map can be fought at
   two times of day; that field is look data too and stays out of
   `trpg-core`. Document both READMEs.
3. **Shading**. A multiply tint alone can't grey a colour, so choose one
   and record it in an ADR (`write-adr`):
   - *(preferred)* the sprite item gains `shade: Option<(grey, tint)>`,
     applied by `app`'s renderer (a small fragment shader in macroquad,
     the same maths in 0232's PNG renderer), one snapshot token per shaded
     sprite; or
   - the importer (0437) writes one pre-shaded copy of the tileset image
     per light into `assets-private/`, and the skin picks the image.
     Simpler to draw, but every new light means re-importing.
   Check the web build draws the same as native (in the browser pane,
   reading canvas pixels back as 0231 did).
4. **Skin**: `SpriteSkin` reads the scene's light (add it to `MapScene`
   as a plain name, ADR-0038: the screen says what, the skin says how)
   and shades terrain sprites only. `every_scene_feature_is_painted`
   (0433) gains the light.
5. **Content**: set each existing map's light. `test_small` stays noon.
   For Chapter 1 (0803, if done) take the time of day from
   `docs/story/chapters/ch01.md`; if the story doesn't say, leave noon and
   ask Nick in the sign-off. Don't invent a time of day.
6. Render the three lights with the bought tiles, **look at them**, send
   them to Nick.

## Acceptance criteria

- [ ] A map with `light: "dusk"` draws its terrain shaded and its units
      unshaded (test on the sprite items: terrain sprites carry the shade,
      unit sprites don't).
- [ ] A map with no `look` or no `light` draws exactly as before this
      ticket (snapshots unchanged).
- [ ] Unknown light name: a validator error naming the file and the name.
- [ ] The shading maths is one pure function with a unit test pinning C3's
      numbers on three sample colours, used by both renderers.
- [ ] Glyph-skin snapshots unchanged; `the_skin_never_changes_the_game`
      passes with a shaded map.
- [ ] Nick was sent the three renders and the per-map list.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `lighting.ron` loading and each error; `MapLook` with and without
  a light; the battle-file override; the shading function.
- Snapshot / integration: Quick Battle on the public fixture tileset under
  dusk; behaviour tests read the `MapScene`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
