---
id: "0029"
title: "Decide: backgrounds behind dialogue scenes"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0021"]
nick_input: decision
completed:
---

# 0029 — Decide: backgrounds behind dialogue scenes

## Context

The dialogue screen (0704) draws two portraits, name plates and a text box on
a black background. Fire Emblem games draw a background picture behind story
conversations (a castle hall, a forest, a village). Nothing in
`docs/design/look-and-feel.md` or the backlog says whether ours should. Found
while listing Chapter 1 gaps (2026-09-29).

Constraints:

- Portraits are bought 48×48 Tiny Tales faces by Mega Tiles (0021, 0711). Nick disliked Claude-drawn
  portraits, so Claude-drawn backgrounds may clash in the same way; say so
  honestly and show them next to the bought portraits.
- Any bought art follows ADR-0032 and the private assets repo (0110).
- **Already bought (2026-10-02):** the Mega Tiles bundle includes
  *Battlebacks Vol.1*: 24 backgrounds, 336×248 pixels, eight outdoor places
  (desert, oasis, two fields, meadows, two mountains, wasteland) each at
  day, dusk and night, and the Tower tileset's three sky pictures (384×512).
  There is nothing indoors and no village or town picture. The tilesets
  (Overworld, Dungeons 1 and 2, Tower) can also be laid out as a room or a
  yard and used as a backdrop. Show both under option C before looking for
  more packs.
- Glyph-drawn art is the game's own style (ADR-0018).

## Nick input

**Decision.** With the `ask-nick` skill, show real mockups of the dialogue
screen with a bought portrait on each option:

- **A. Keep black** (today).
- **B. Glyph-drawn backgrounds** (the `ascii-art` skill), dimmed so the
  portraits stand out.
- **C. Bought background art** matching the portrait style (search for packs
  under allowed licences; name the price and licence). Look first at the
  artist 0021 picked (Mega Tiles, which sells tile and terrain packs), then
  others such as CaptainSkolot, who sells background packs; say honestly
  if the styles clash.
- **D. No picture, just a place line** (for example "The old fort, at dusk") above
  the text box.
- **E.** "Describe your own."

Sub-question: if backgrounds, how many does Chapter 1 need (one per
location in `docs/story/chapters/ch01.md`)?

## Scope

**In:**
- The mockups and the decision, recorded in `docs/design/look-and-feel.md`
  with Nick's words.
- If Nick picks B, C or D: create the implementation ticket (a `@background`
  or `@place` directive for `.dlg` scripts, like 0710's `@music`, plus the
  art or text), with the `write-ticket` skill, and say whether it blocks the
  playtest (0804).

**Out (do not do):**
- Implementing it.
- Buying anything (Nick's call and money).

## Implementation steps

1. Build mockups with real portraits (0711 format).
2. Run the decision; record it.
3. Create the follow-up ticket if needed.

## Acceptance criteria

- [ ] `look-and-feel.md` records the choice and Nick's words.
- [ ] A follow-up ticket exists if the answer isn't A.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (design decision; docs only).

## Completion notes

*(Filled in by the session that completes the ticket.)*
