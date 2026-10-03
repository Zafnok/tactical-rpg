---
id: "0235"
title: "Language packs cover data names, tips and dialogue lines"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0233", "0717"]
nick_input: none
completed:
---

# 0235 — Language packs cover data names, tips and dialogue lines

## Context

After 0233/0234 a language pack can replace screen text. The rest of what
a player reads comes from data: `name` and description fields in
`assets/data/*.ron`, the names table, tips, captions and dialogue.
[ADR-0045 §2](../../docs/adr/0045-languages-text-by-key-and-line-ids.md)
gives each a key; 0717 gave dialogue lines ids. English stays in the data
files; a pack overlays it.

## Nick input

None.

## Scope

**In:** `data.ron` and `dialogue/<file>.ron` in a pack; lookups for every
data text; the dialogue screen and tips showing the pack's text; the lead
tokens in another language; the test pack extended.

**Out (do not do):** any Japanese text (0719); wide glyphs and Japanese
wrapping (0236, 0237); the credits' work titles and authors (names of
works stay as they are); changing any English.

## Implementation steps

1. List every player-facing text field in `crates/content/src` (items,
   classes, skills, spells, arts, terrain, characters' titles if any,
   tips, `names.ron`, battle and chapter titles, captions). Put the list
   in `assets/lang/README.md` with each one's key form:
   `<file>.<id>.<field>`, `names.<id>`, `tips.<id>.title|text`,
   dialogue line id, `caption.<scene>_<8 hex>` for `@caption` text
   (hash as in 0717).
2. `trpg_content::lang`: load `data.ron` and `dialogue/*.ron` entries;
   the same unknown-key, stale and missing rules as 0233. `Lang::status`
   includes them.
3. Lookups in `content`, taking a `LangCode`: one function per kind
   (`lang.item_name(code, &ItemDef)`, `lang.line(code, &LineId)`, …) so
   callers can't build keys by hand. `ui` reaches them through `Ctx`
   helpers next to `Ctx::text`.
4. Replace every place `ui` reads `.name` (and descriptions) directly for
   display with the helper. `core` keeps using ids only; where `core`
   returns a name for display today, return the id and let `ui` look it
   up (ADR-0004: `core` knows no language).
5. **Name tokens** in a translated line resolve against the pack's
   `names.<id>` (falling back to English). `{N:…}` capitalises only when
   the result starts with a Latin letter.
6. **Lead tokens.** `{lead}` works in any language. A translated line may
   not need `{they}`-style tokens; they stay allowed (they give the
   English words). For languages where the lead's gender changes more
   than a pronoun, an entry may carry `text_m` and `text_f` instead of
   `text`; the dialogue screen picks by the lead's gender. Validate that
   an entry has either `text` or both.
7. Validation of a pack's dialogue text: the same tokens rule as English
   (only known tokens; every `{n:…}` id exists), and the length limits
   counted in cells via one `text_width` function (today 1 per char;
   0236 makes it glyph-aware). The plain-ASCII and no-written-names
   rules apply to English only.
8. Extend the test pack with a few data names, a tip and one scene of
   `assets/dialogue/test.dlg`; Harness tests show them.
9. `cargo xtask lang-status` covers the new kinds and, for an orphaned
   dialogue entry, prints the new line in the same scene whose text is
   nearest, with the old translation.

## Acceptance criteria

- [ ] In the test pack's language, an item name, a class name, a tip and a dialogue line show translated; everything else shows English (Harness tests).
- [ ] A name token inside a translated line uses the pack's name (test).
- [ ] An entry with `text_m`/`text_f` shows the right one for each lead (test).
- [ ] Rewording an English line makes its entry an orphan that `lang-status` pairs with the new line (test).
- [ ] No `ui` code reads a data `name` field for display except through the helpers (a `check-text` rule or a grep test).
- [ ] English snapshots are unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loading and validation per kind; token resolution; gender variants.
- Snapshot / integration: dialogue and an info screen in the test pack.

## Completion notes

