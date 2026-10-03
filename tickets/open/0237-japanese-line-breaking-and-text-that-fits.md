---
id: "0237"
title: Japanese line breaking, and screens whose text must fit
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0235", "0236"]
nick_input: none
completed:
---

# 0237 — Japanese line breaking, and screens whose text must fit

## Context

After 0236 Japanese glyphs draw, two cells each. But wrapping breaks at
spaces, which Japanese doesn't have, and some layouts were sized for an
English word. [ADR-0045 §4](../../docs/adr/0045-languages-text-by-key-and-line-ids.md)
sets the rule: break between any two characters, except before closing
punctuation and after opening brackets.

## Nick input

None.

## Scope

**In:** one wrapping function used everywhere text wraps; the dialogue
box, tips, descriptions and the rewind notes using it; a check that no
pack text overflows its place.

**Out (do not do):** translating (0719); new layouts for Japanese (if a
place can't fit, report it; a redesign is its own ticket); furigana;
vertical text.

## Implementation steps

1. Find every wrapping routine in `crates/ui/src` (the dialogue screen's
   paging, tips, `screens/battle/rewind.rs`'s `wrap`, description
   panels). Replace them with one `wrap(text, width_cells) -> Vec<String>`
   in a `ui` text module, measuring with 0236's `text_width`.
2. Breaking rules: Latin words break at spaces as today. Between two wide
   characters a break is allowed anywhere, except: never start a line
   with `、。，．・：；？！）」』】〕ー` or a small kana (`ぁぃぅぇぉっゃゅょ`
   and their katakana); never end a line with `（「『【〔`. Keep a run of
   Latin letters or digits inside Japanese text together. A key name
   filled into a placeholder is never split.
3. The dialogue box's "3 lines" paging uses the same function; a
   Japanese line pages at the same 3 lines.
4. `content`'s length limits for pack text (0235 step 7) use cells with
   the real widths: a dialogue line at most 200 cells, a reply 60, a
   `lead:` line 40, tips 3 lines of 60.
5. `cargo xtask lang-status <code>` gains an **overflow** list: for every
   `ui.ron` key that is drawn in a fixed-width place, whether the pack's
   text fits. Give such keys a width in `en/ui.ron` (an optional
   `max_cells` per key, set from the layout); the tool compares.
6. Fix any layout 0234 noted as sized for one English word, where the fix
   is measuring instead of a constant.

## Acceptance criteria

- [ ] A Japanese paragraph wraps within the width, with no line starting with `。` or `、` and none ending with `「` (unit tests with real sentences).
- [ ] English wrapping is unchanged (existing snapshots).
- [ ] A property test: every wrapped line is at most the width, and joining the lines gives back the text (apart from break spaces).
- [ ] `lang-status test` reports an entry made too long on purpose as overflow.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each breaking rule; mixed Latin and Japanese; placeholders.
- Property: width and round trip.
- Snapshot / integration: the dialogue screen showing a Japanese line from the test pack.

## Completion notes

