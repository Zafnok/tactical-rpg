---
id: "0718"
title: "Japanese translation pipeline: glossary, speech styles, the back-translation check"
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0042", "0235"]
nick_input: none
completed:
---

# 0718 — Japanese translation pipeline: glossary, speech styles, the back-translation check

## Context

The Japanese option is machine-translated by Claude, chapter by chapter,
until a human translation replaces it (`docs/design/voices-languages-and-script.md`).
ADR-0011 showed that LLM writing quality comes from process: a bible,
character sheets, a separate critique pass. Translation needs the same,
more so because nobody on the project can read the result
([ADR-0045 §6](../../docs/adr/0045-languages-text-by-key-and-line-ids.md)).
This ticket sets the process up; 0719 uses it on Chapter 1.

## Nick input

None. (0042 answered what a player sees: katakana names or not, the
title.) Nick can't check Japanese, so no sign-off is asked.

## Scope

**In:** the documents and the skill text a translating session follows;
the Japanese names table and game-term glossary; the pack's `lang.ron`.

**Out (do not do):** translating scripts or screen text (0719); code
(0233–0237 built it). Don't copy another game's translated text; other
games are references for *conventions* only (ADR-0013).

## Implementation steps

1. `docs/story/ja/README.md`: the pipeline, as steps a later session
   follows per chapter: read the glossary and the speakers' style sheets
   → translate a scene at a time into `assets/lang/ja/dialogue/<file>.ron`
   → a **separate** session translates the Japanese back to English
   without seeing the original, then compares the two and lists lines
   whose meaning, tone or speaker register drifted → revise → run
   `cargo xtask lang-status ja` (no missing, stale or overflow).
2. `docs/story/ja/glossary.md`: every game term on screen (stats, weapon
   kinds, classes, menu verbs, phases, difficulty modes, magic terms from
   `docs/design/`) with its Japanese, chosen to match what Japanese
   SRPG players already know (Fire Emblem's and Tactics Ogre's Japanese
   releases are the reference for conventions: 力, 守備, 命中, 必殺…).
   One term, one translation, everywhere. Note which stat labels stay
   Latin (`HP`, `Lv`).
3. `docs/story/ja/names.md`: every id in `assets/data/names.ron` with its
   Japanese form per 0042's answer (katakana, with the reading chosen
   from the English name's pronunciation; note doubtful ones), entered in
   `assets/lang/ja/data.ron` as `names.<id>`. Names stay renameable: the
   rename rule in `docs/story/names.md` gains a line (a rename makes the
   Japanese entry stale).
4. `docs/story/ja/styles.md`: per character in
   `docs/story/characters/*.md`, from its *Voice notes*: first-person
   word (俺 / 私 / わし…), how they address others, politeness level,
   sentence endings, three sample lines translated. For the lead: both
   genders, short and neutral, and when a line needs `text_m` / `text_f`
   (0235).
5. Token rules for translators, in the README: `{lead}` and `{n:…}` are
   kept; `{they}`-style tokens are usually dropped in Japanese (the
   subject is left out); key-name placeholders are kept as they are.
6. `assets/lang/ja/lang.ron`: name `日本語`, `made_by: Machine`.
7. Add a "Translating" section to the `story-writing` skill pointing at
   the README, and a row for it in ADR-0011's pipeline list by way of a
   one-line note in `docs/story/README.md` (ADR-0045 already amends
   ADR-0011).

## Acceptance criteria

- [ ] The glossary covers every term in `assets/lang/en/ui.ron` and the data files that 0719 will translate (list any left for 0719 with a reason).
- [ ] Every `names.ron` id has a Japanese entry; `cargo xtask lang-status ja` shows no missing `names.*` key.
- [ ] Every Chapter 1 speaker has a style sheet.
- [ ] The pipeline README can be followed by a session that has read nothing else (try it on one scene of `test.dlg` and note the result).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The all-assets test validates `assets/lang/ja/`.

## Completion notes

