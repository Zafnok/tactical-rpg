---
id: "0039"
title: "Decide: how sprite units show their side and \"acted\", the zoom key, and what the playtest waits for"
type: design-decision
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0038"]
nick_input: decision
completed:
---

# 0039 — Decide: sprite units' marks, the zoom key, playtest timing

## Context

In 0038 (2026-10-02) Nick chose to draw the battle map with the bought
Tiny Tales tiles and unit sprites, with per-map lighting and a zoom toggle
(`docs/design/look-and-feel.md`, *Battle map: bought tiles and unit
sprites*). These questions were left open. The first three were asked in
the same conversation, on 2026-10-02; if Nick answered there, record the
answers and ask only the rest.

The mockups are in `spike-renders/` in the bought-art folder on Nick's
machine (see 0038's Completion notes). Make new ones the same way: the
game's font atlas and palette, at 2×, never committed (ADR-0032).

## Nick input

**Decision** (`ask-nick`, at most three questions per message, with
renders).

**Asked on 2026-10-02:**

1. **How a sprite unit shows whose side it is on, and that it has acted.**
   With initials the letters were blue or red; a sprite has its own
   colours. Render D showed: HP bar only; a 1-pixel outline in the side's
   colour (used in renders B, C, C2, C3); a tinted tile; a small corner
   mark. Acted units were drawn grey and darker. Compare Fire Emblem (GBA):
   every unit's clothes are recoloured blue or red, and an acted unit turns
   grey. Recolouring each bought sprite's clothes is a per-sprite job and
   changes the characters' own colours, so it wasn't offered as the
   default; say so if Nick asks.
2. **The zoom toggle's default key and button** (`controls.md`; the
   `keyboard-input` skill: default keys are Nick's). Free keys next to the
   others: `Q` in the right-handed layout and `P` in the left-handed one
   (finger mirrors); the right trigger (`RT` / `R2` / `ZR`) on a
   controller (the left trigger is Rewind). And whether it is an optional
   action (may be left without a key), like Unit info and Rewind.
3. **Does the Chapter 1 playtest (0804) wait for the bought-art map?**
   Waiting means 0432, 0433, 0436 and 0437 (and 0438, 0439 if he wants
   them in) join 0804's `blocked_by`, next to 0110 and 0413 which are
   already there.

**Still to ask:**

4. **Do sprites walk?** Each map sprite has three walking frames in four
   directions. Options: stand still facing the camera (as in the renders);
   step on the spot while waiting, as in Fire Emblem; walk along the path
   when moving, turning to face the way they go. Show short animated
   mockups (as 0036 does).
5. **May players pick the glyph look?** It stays in the game (ADR-0038) as
   what the public repository shows. Options: bought art only; an Options
   entry "Map look: pictures / glyphs" (0805).
6. **Under an effect:** the glyph look tints the unit's background with the
   `effect` colour. What does a sprite unit show (a small mark, a tinted
   tile, an outline colour)? Render two or three.
7. **Face or bust in dialogue.** 0021 chose the 48×48 faces partly because
   the large portrait was thought to have one expression; the bought 80×80
   busts have the same 8. Render H showed a face at 5× beside a bust at 3×
   (both 240 pixels). Faces stay the rule unless Nick picks busts; if he
   does, update 0711 and 0706.

## Scope

**In:** the questions above, recorded in `look-and-feel.md` (questions 1,
4–7) and `controls.md` (question 2: the default-keys table, the controller
table and the required/optional lists); 0804's `blocked_by` and
`docs/ROADMAP.md` per question 3; the lines in 0436, 0439, 0711 and 0706
that the answers change.

**Out (do not do):** building any of it; buying anything.

## Implementation steps

1. Read what Nick already answered (the 2026-10-02 conversation; check
   `look-and-feel.md` and `controls.md` for answers recorded since).
2. Render what is missing (questions 4 and 6 need new mockups).
3. Ask; record Nick's words and the rules; update the tickets named in
   *Scope*.

## Acceptance criteria

- [ ] Each question has an answer in `look-and-feel.md` or `controls.md`
      with Nick's words, or a written note that Nick put it off and until
      when.
- [ ] 0436, 0439, 0804 (and 0711, 0706 if question 7 changes them) match
      the answers.
- [ ] `cargo xtask ticket-lint` and `typos` pass.

## Tests required

- None (docs only).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
