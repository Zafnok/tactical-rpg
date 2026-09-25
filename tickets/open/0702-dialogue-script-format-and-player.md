---
id: "0702"
title: Dialogue script format (.dlg), parser/validator, dialogue player model
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0302"]
nick_input: none
completed:
---

# 0702 — Dialogue script format and player model

## Context

[ADR-0005](../../docs/adr/0005-data-driven-content.md) chose a line-based,
prose-first script format so LLM writers produce it directly. This ticket
defines it precisely, parses and validates it, and provides a pure
`DialoguePlayer` that the dialogue screen (0704) renders.

## Nick input

None.

## Scope

**In:** format spec in `assets/dialogue/README.md`, parser + validator in
`content`, `ui::dialogue::DialoguePlayer` (state machine, no drawing),
one example scene.

**Out:** drawing (0704), portraits (0703), battle triggers (0705).

## Implementation steps

1. **Format** (write the spec with examples in `assets/dialogue/README.md`):
   ```
   # comment
   @scene ch01_opening                 # starts a scene; ids unique across all files
   @caption Village of Heth, dusk      # optional location/time caption
   @left  ana neutral                  # place character (id, expression) on a side
   @right bors angry
   bors: You're late.                  # speaker must be on screen
   ana[happy]: Better late than... well.   # [expr] changes expression, then speaks
   > The rain had not stopped for three days.   # narration (no speaker)
   @right clear                        # remove
   @right mira surprised               # enter
   @end
   ```
   Multiple scenes per file allowed. Blank lines ignored. A text line may
   continue onto following lines indented by two spaces (joined with a space).
2. **Parser** → `Scene { id, steps: Vec<Step> }` where `Step` = `Caption`,
   `Place { side, character, expression }`, `Clear { side }`,
   `Say { speaker, expression: Option, text }`, `Narrate { text }`.
3. **Validator** (all errors, with file:line): unknown character id (against
   `characters.ron`); speaker not on screen; same character on both sides;
   unknown expression (checked against the character's portrait expressions when
   the portrait exists — 0703 adds that cross-check; until then accept the five
   standard names); text > 200 chars; duplicate scene ids; missing `@end`;
   non-ASCII punctuation (smart quotes) → error suggesting the ASCII form.
4. **`DialoguePlayer`** (in `ui`, pure): `new(scene)`, `current() -> View { left: Option<(CharId, Expr)>, right, speaker: Option<Side>, text: Option<&str>, caption: Option<&str>, narration: bool }`,
   `advance()` processes non-text steps until the next text step (so each
   `advance` = one text box), `is_finished()`.
5. Example: `assets/dialogue/test.dlg` with a short scene using placeholder
   characters; add dialogue to `Content` and the all-assets test.

## Acceptance criteria

- [ ] Spec documented with a full example; the `story-writing` skill's format section points to it (edit the skill if needed).
- [ ] Every validator error has a test with the exact message.
- [ ] `DialoguePlayer` walks the example scene correctly (test lists each `View`).

## Tests required

- Unit: parser per line type; continuation lines; validator errors.
- Property: pretty-print → parse round-trip for random scenes; `DialoguePlayer` visits every `Say`/`Narrate` exactly once in order.

## Completion notes

