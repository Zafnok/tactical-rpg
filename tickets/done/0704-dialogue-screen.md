---
id: "0704"
title: "Dialogue screen: two portraits, name plates, typewriter text box"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: done
blocked_by: ["0702", "0703"]
nick_input: sign-off
completed: 2026-09-28
---

# 0704 — Dialogue screen

## Context

Renders a `DialoguePlayer` (0702) with portraits (0703): two characters on
screen, speaker bright, listener dimmed, text box beneath
([ADR-0018](../../docs/adr/0018-visual-style-v2.md); layout Nick approved in
`docs/screenshots/0011-conversation.png`).

## Nick input

**Sign-off:** Nick reads a test scene and comments on readability and pacing.

## Scope

**In:** `DialogueScreen` (full-screen and overlay-on-map variants), word wrap,
typewriter reveal, advance/fast-forward/skip, caption and narration styles.

**Out:** triggers from battle (0705), voice/sound.

## Implementation steps

1. Layout (100×32): portraits are 32×16 cells, each in a 34×18 frame at left
   `x=1` and right `x=65`, `y=1`; the **speaker's frame is double-line** in
   `panel_border_focus`, the listener's single-line; name plates on row 19; the
   text box is a single-line box across the bottom (rows 21–27, full width), 3
   text lines, with the speaker's name on the top border. The right-hand
   portrait is drawn mirrored so it faces the speaker. Caption, if any,
   top-centre.
   Narration: portraits dimmed, text centred in the box, italic-feel via `text_dim`.
2. Speaker at full brightness; listener `dim(0.45)`; empty side draws nothing.
3. `word_wrap(text, width) -> Vec<String>`: splits on spaces, never splits
   words ≤ width, hard-splits longer words; text longer than 3 lines paginates
   into multiple boxes (each confirm advances a page).
4. Typewriter: chars/sec from a setting (default 60; 0805 will expose it).
   Confirm while revealing → reveal all; Confirm when revealed → next;
   hold Confirm → fast-forward (≈ ×6); `Cancel` → "Skip scene? f yes / d no".
   A small `▼` blinks in the box corner when waiting.
5. Overlay variant (`is_overlay() == true`) for in-battle lines: map stays visible
   behind, portraits and box drawn over it.
6. Debug: F2 menu entry "Play test scene".

## Acceptance criteria

- [x] Test scene plays end-to-end in both variants.
- [x] Word wrap property tests pass.
- [x] Harness: `"f f f"` through the test scene reaches the end and pops; skip-confirm path works.
- [x] Snapshots: mid-reveal, fully revealed, narration, overlay on the battle map.

## Tests required

- Property: `word_wrap` — no line exceeds width; joining lines' words == original words (for texts without over-long words).
- Harness + snapshots as above.

## Completion notes

- **Screen:** `trpg_ui::screens::DialogueScreen` (`new` = full-screen,
  `overlay` = drawn over the screen below). Layout exactly as in step 1.
  It pops when the scene ends or is skipped. 0705 pushes it from battle.
- **Word wrap:** `trpg_ui::widgets::word_wrap`, with property tests (no
  line too wide, words kept in order, no character lost, lines filled
  greedily). Text boxes are 92 characters wide, 3 lines a page.
- **Text speed:** `Ctx::text_speed`, default 60 characters/second, for 0805
  to move into the saved settings. Holding Confirm reveals 6× faster.
- **Space advances too (Nick asked while the PR was open):** the End Turn
  key (Space in both layouts) does everything Confirm does in a scene:
  reveal, next box, hold to fast-forward, and "yes" to the skip question.
  The box corner and skip question still name only the Confirm key.
- **Debug menu:** two entries, not one: "Play test scene" (full-screen) and
  "Play test scene (overlay)". The overlay entry replaces the debug menu, so
  pressing F2 (the debug key since main moved it from F12) during a battle plays the scene over that battle's map.
- **Screenshots:** `docs/screenshots/0704-dialogue.png`,
  `0704-narration.png`, `0704-dialogue-overlay.png`.

Small look choices the ticket left open (Nick can change any of them):

- Name plates: the speaker's name in normal text, the listener's dimmed,
  on a panel-coloured strip under each frame.
- The `▼` has the Confirm key's name before it (`f ▼`), as in the approved
  mockup. The `▼` blinks every half second; the key name stays.
- The caption is centred on the top row in the highlight colour, and stays
  until the scene changes it (0702 already decided that).
- The skip question appears inside the text box: "Skip scene?" and
  "f yes / d no".
- A character with no portrait yet (e.g. `test_archer`) gets an empty frame
  and a name plate.
- In the overlay, the rows below the text box are blanked, because the
  battle's key help there names keys that do nothing during a scene.
- Holding Confirm only speeds up the reveal. It doesn't move to the next
  box by itself; each box still takes a press.

Gameplay rules decided: none. No follow-up tickets.
