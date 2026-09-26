---
id: "0703"
title: ASCII portrait file format, loader, renderer and debug viewer
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0205", "0302"]
nick_input: none
completed:
---

# 0703 — Portrait format, loader, renderer, viewer

## Context

Nick wants character portraits, two at a time, in story scenes. In 0011 he
chose **32×32 shaded pixel art** drawn with half-block cells: rules in
[ADR-0018](../../docs/adr/0018-visual-style-v2.md),
[`look-and-feel.md`](../../docs/design/look-and-feel.md) and the `ascii-art`
skill; the style sample is `docs/screenshots/0011-portrait-expressions.png`.

## Nick input

None (art is judged in 0706).

## Scope

**In:** `.portrait` format spec, parser/validator, `draw_portrait(buf, pos, portrait, expr, dim)`,
a debug portrait viewer screen (F12 menu), two placeholder portraits.

**Out:** real character art (0706), dialogue screen (0704).

## Implementation steps

1. **Format** (spec + example in `assets/portraits/README.md`):
   ```
   (
     character: "ana",
     size: (32, 32),       # pixels; drawn as 32×16 cells
     colors: { 'h': "hair_brown", 's': "skin_light", 'q': "skin_light_mid", 'a': "armor_steel", 'e': "eye_green" },
   )
   === neutral
   <32 lines of exactly 32 colour keys; '.' = transparent>
   === happy
   ...
   ```
   Palette names may be new: add portrait colours to `palette.ron` as needed
   (skin tones, hair colours, metals…).
2. Parser/validator (file:line errors): size mismatch, unknown colour key,
   unknown palette name, missing required expressions (`neutral, happy, angry,
   sad, surprised`), duplicate expressions, `.` reserved for transparent.
3. Cross-check in content: dialogue expressions (0702) must exist in the
   speaker's portrait when a portrait exists.
4. `ui::portrait::draw_portrait(buf, x, y, &Portrait, expr, dim: f32, mirror: bool)`:
   each cell is `▀` with fg = top pixel, bg = bottom pixel (`▄` / space where a
   pixel is transparent, over the given background); `dim` lerps toward the
   background; `mirror` reverses each row (exact for pixel art).
5. Debug viewer (debug builds): list portraits, `h/l` switch expression,
   `j/k` switch character, shows name + expression name.
6. Two placeholder portraits (`test_lord`, `test_knight`), clearly marked as
   placeholders, following the `ascii-art` skill.

## Acceptance criteria

- [ ] Spec documented; placeholders load; all-assets test passes.
- [ ] Validator errors tested.
- [ ] Viewer works; screenshot in PR.

## Tests required

- Unit: parser/validator; draw clipping.
- Snapshot: each placeholder portrait neutral + one other expression; dimmed variant; mirrored variant.

## Completion notes

