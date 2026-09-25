---
id: "0704"
title: "Dialogue screen: two portraits, name plates, typewriter text box"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0702", "0703"]
nick_input: sign-off
completed:
---

# 0704 — Dialogue screen

## Context

Renders a `DialoguePlayer` (0702) with portraits (0703): two characters on
screen, speaker bright, listener dimmed, text box beneath
([ADR-0012](../../docs/adr/0012-visual-style.md)).

## Nick input

**Sign-off:** Nick reads a test scene and comments on readability and pacing.

## Scope

**In:** `DialogueScreen` (full-screen and overlay-on-map variants), word wrap,
typewriter reveal, advance/fast-forward/skip, caption and narration styles.

**Out:** triggers from battle (0705), voice/sound.

## Implementation steps

1. Layout (100×32): portraits at left `x=4` and right `x=72`, `y=4`, each in a
   single-line frame; name plates under each portrait; the text box is a
   double-line box across the bottom (rows 22–29), 3 text lines, ~70 chars wide,
   with the speaker's name on the top border. Caption, if any, top-centre.
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
6. Debug: F12 menu entry "Play test scene".

## Acceptance criteria

- [ ] Test scene plays end-to-end in both variants.
- [ ] Word wrap property tests pass.
- [ ] Harness: `"f f f"` through the test scene reaches the end and pops; skip-confirm path works.
- [ ] Snapshots: mid-reveal, fully revealed, narration, overlay on the battle map.

## Tests required

- Property: `word_wrap` — no line exceeds width; joining lines' words == original words (for texts without over-long words).
- Harness + snapshots as above.

## Completion notes

