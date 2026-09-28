---
id: "0702"
title: Dialogue script format (.dlg), parser/validator, dialogue player model
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: done
blocked_by: ["0302"]
nick_input: none
completed: 2026-09-28
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

**Out:** drawing (0704), portraits (0703), battle triggers (0705), lead reply
choices and `{lead}`/pronoun tokens (0708, which extends this format; keep the
parser easy to extend with block steps).

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

- [x] Spec documented with a full example; the `story-writing` skill's format section points to it (edit the skill if needed).
- [x] Every validator error has a test with the exact message.
- [x] `DialoguePlayer` walks the example scene correctly (test lists each `View`).

## Tests required

- Unit: parser per line type; continuation lines; validator errors.
- Property: pretty-print → parse round-trip for random scenes; `DialoguePlayer` visits every `Say`/`Narrate` exactly once in order.

## Completion notes

- **Format spec**: `assets/dialogue/README.md` (full example, line table,
  rules). A test parses that example so the spec can't drift from the
  parser. The `story-writing` skill's format section now points to it.
- **Parser + validator**: `trpg_content::dialogue` (`parse_dlg`,
  `check_scene`, `check_duplicates`, `print_scene`, `load`). Dialogue is in
  `Content::dialogue` (`DialogueTable`), so the all-assets test covers it.
  Every error has a test with its exact message; errors come out sorted by
  file and line.
- **Player**: `trpg_ui::dialogue::DialoguePlayer` (`new`, `current() -> View`,
  `advance`, `is_finished`). It owns its `Scene` (a clone) so the 0704 screen
  can hold it without borrowing `Content`. `View` gives portraits as
  `Portrait { character, expression }`.
- **Example**: `assets/dialogue/test.dlg`, scene `test`, with the placeholder
  characters (`test_lord`, `test_knight`, `test_archer`).

Format choices the ticket left open (technical, no gameplay effect):

- Comments are whole lines only (`#` at the start). A trailing `# ...` on a
  directive is an error, so it can't silently become part of a caption.
- A caption stays until the next `@caption` (0704 decides how long it is
  drawn).
- `@left`/`@right` always need an expression (no default); `clear` is
  reserved and can't be a character id.
- Beyond the smart-quote/dash/ellipsis suggestions, any other non-ASCII or
  control character in text (e.g. `é`, a tab, a non-breaking space) is an
  error too, since the font may not have it.
- Extra checks, each with its own message: invalid ids (lowercase, digits,
  `_`), unknown directives, wrong directive arguments, empty text, lines
  outside a scene, `@end` without `@scene`, misplaced indented lines, and a
  scene with no speech or narration.
- A continuation is any line indented more than column 1 that follows speech
  or narration (blank lines and comments in between are ignored). For 0708's
  `@choice` blocks, the parser reads line by line with an open-scene state,
  so a block step can be added as another state.

No gameplay rules were decided. No follow-up tickets.
