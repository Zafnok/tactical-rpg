---
id: "0233"
title: "Screen text by key from a language file (the mechanism, and the title screen)"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-03
---

# 0233 — Screen text by key from a language file (the mechanism, and the title screen)

## Context

For a Japanese option (`docs/design/voices-languages-and-script.md`),
screens must stop writing English as string literals.
[ADR-0045 §1–2](../../docs/adr/0045-languages-text-by-key-and-line-ids.md):
English screen text moves to `assets/lang/en/ui.ron`, screens ask for it
by key, other languages are packs that overlay it. This ticket builds the
mechanism and converts one screen as the pattern; 0234 converts the rest.

## Nick input

None. Nothing a player sees changes.

## Scope

**In:** `trpg_content::lang`; `assets/lang/en/ui.ron`; `Ctx::text`; the
language in `Ctx`; the title screen converted; a test pack; a check that
counts the literals left.

**Out (do not do):** other screens (0234); data names, tips and dialogue
(0235); wide glyphs (0236); the Options row and saving the choice (0825);
any Japanese text. Help text that names keys keeps going through
`ctx.help_keys()` (ADR-0036); only the words around the key names move.

## Implementation steps

1. `crates/content/src/lang.rs`: `LangCode` (a short lowercase id, `en`,
   `ja`), `LangInfo { name, made_by: Machine | Human(String) }`,
   `Entry { key, source, text }`, `LangPack`, and `Lang` holding English's
   `ui` table and every pack. Load `assets/lang/en/ui.ron` (a map of key
   → text) and each `assets/lang/<code>/{lang.ron, ui.ron}` into
   `Content::lang`. Validation: keys are lowercase letters, digits, `_`
   and `.`; no duplicate keys; a pack entry with an unknown key is an
   error; a pack entry's placeholders must be the same set as the
   English text's.
2. Lookup: `Lang::text(code, key) -> &str` returns the pack's text when
   it has an entry whose `source` equals today's English, else English.
   `Lang::status(code)` returns the missing and the stale keys.
3. Placeholders: `{name}` filled from `&[(&str, &dyn Display)]`. Reuse
   or extend the function that fills tips' placeholders
   (`fill_placeholders`) rather than writing a second one; key-name
   placeholders (`{Confirm}`) keep working inside the same text.
4. `crates/ui/src/screen.rs`: `Ctx` gets `lang: LangCode` (default `en`),
   `fn text(&self, key: &str) -> &str` and `fn text_with(&self, key,
   args) -> String`. In debug builds a key missing from English panics
   with the key's name (as unknown audio cues do).
5. Convert `crates/ui/src/screens/title.rs`: every literal the player
   reads becomes a key (`title.new_game`, …) in `en/ui.ron`. Snapshots
   must not change.
6. A test pack `assets/lang/test/` (id `test`, never offered to players;
   loaded only when `Ctx::debug_tools`) that upper-cases the title
   screen's text, with one stale and one missing entry; a Harness test
   shows the title in it and checks the fallbacks.
7. `cargo xtask lang-status <code>`: prints missing and stale keys, exit
   code 0 either way.
8. `cargo xtask check-text`: counts string literals passed to
   `GlyphBuffer::print*` and the text widgets in `crates/ui/src`
   (outside tests and `debug`), prints them by file, and fails if the
   count is above a number stored in the tool. Set the number to today's
   count after step 5; 0234 lowers it to zero. Model it on
   `crates/xtask/src/check_keys.rs`. Add it to the `run-gates` skill and
   CI where `check-keys` runs.
9. Set ADR-0045's status to `Accepted`; record any change to §1–2.

## Acceptance criteria

- [x] The title screen has no player-facing literal; its snapshots are unchanged.
- [x] With the test pack, the title shows the pack's text; a stale or missing entry shows English (Harness test).
- [x] A pack entry with an unknown key, or with different placeholders, fails validation with file and key (unit tests).
- [x] An unknown key panics in debug builds (test).
- [x] `cargo xtask lang-status test` lists the one stale and one missing key.
- [x] `cargo xtask check-text` passes and fails when a new literal is added (test).
- [x] `core` has no dependency on `lang` (ADR-0004).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loading, validation errors, lookup and fallback, placeholders.
- Snapshot / integration: title screen in `en` (unchanged) and in `test`.

## Completion notes

Done as planned. Nothing a player sees changes; no gameplay rule was
decided here.

- `trpg_content::lang`: `LangCode`, `LangInfo`, `MadeBy`, `Entry`,
  `LangPack`, `Lang` (`text`, `status`, `has`, `english`, `pack`, `codes`),
  loaded into `Content::lang` from `assets/lang/`.
- `Ctx::lang`, `Ctx::text(key)`, `Ctx::text_with(key, args)`. The filling
  is `tips::fill_text`, which `fill_placeholders` now calls; it fills in
  one pass, so a value is never filled a second time.
- The title screen reads every text by key. Its help line is one text,
  `{Cursor} move · {Confirm} select · {Cancel} back`, so a translation can
  reorder it. `TitleScreen::new` and `with_quick_battle` take the `Ctx`
  (for the labels), and the menu is labelled again at each refresh, so it
  follows a change of language. Existing snapshots are unchanged.
- `assets/lang/test/`: English in capitals; `title.subtitle` is stale and
  `title.credits` is missing, on purpose.
- `cargo xtask lang-status <code>` and `cargo xtask check-text` (221
  literals today), the latter in CI's `tickets` job and in `run-gates`.
- ADR-0045 is `Accepted`, with what this ticket settled marked *0233*.

Deviations and details settled:

- A pack entry's placeholders are checked against its own `source`, not
  today's English. Otherwise adding a placeholder to an English text
  would fail the gate for every pack, which ADR-0045 says must not happen.
- The test pack is always in the bundle; `Ctx::text` ignores it unless
  `debug_tools` is on (rather than not loading it).
- Any file in a language's directory other than `lang.ron` and `ui.ron`
  is an error until 0235 adds `data.ron` and `dialogue/`.
- `check-text` counts more than literals passed straight to `print`: most
  screens keep their text in `&str` constants, so those count too when
  they look like prose (a capital or a space). A literal the rules get
  wrong (a picture drawn from text rows, a button's name) is let through
  with a `// check-text: not player text` comment; 0234 will need a few.
- The title's `Continue` error notice comes from the save code, so it is
  0234's.
