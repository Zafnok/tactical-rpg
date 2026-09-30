---
id: "0709"
title: "Names table and name tokens, so story names can be renamed in one place"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: done
blocked_by: ["0702"]
nick_input: none
completed: 2026-09-30
---

# 0709 — Names table and name tokens

## Context

At 0701 gate 1 (2026-09-28) Nick said he isn't sold on the story's names and
asked that they be "variables in some way so that when I come back and am like
yea I don't like The Crown of Ardeval faction name, or Brennmark name, or
Emeric name, etc. it's easy to revise the scripts" (quoted in
`docs/story/bible.md`).

0701 created [`docs/story/names.md`](../../docs/story/names.md): every
character, place, faction and story term has a **stable id** (`retainer`,
`king`, `place.brennmark`, `faction.crown`, …) and a current display name.
Character ids are **role ids**, not names, so a rename never moves a speaker
id. The `.dlg` format from 0702 (`assets/dialogue/README.md`,
`trpg_content::dialogue`) has no way to refer to a name by id yet, and
`assets/data/characters.ron` stores display names inline. This ticket adds
the game-side half: one names table, and tokens in dialogue text.

Related: 0708 adds `{lead}` and pronoun tokens to `.dlg` text. Pick a syntax
that can't clash with those (see step 2).

## Nick input

None.

## Scope

**In:**
- `assets/data/names.ron`: id → display name, seeded with every row of
  `docs/story/names.md` (characters, places, factions, gods, terms, minor
  names). It's the only place a display name is written in game data.
- Named characters get their display name from the table: `characters.ron`
  entries reference a name id (or use their character id as the name id)
  instead of a literal `name`. Map labels (`map_label`) stay explicit, since
  they're two letters chosen per character.
- A name token in `.dlg` text lines (`Say`, `Narrate`, `Caption`), e.g.
  `{n:king}` → "Emeric", resolved when the scene is shown (in
  `DialoguePlayer::current()` or the equivalent), so stored scenes are
  unchanged.
- Validator checks (file:line, each with a test and its exact message):
  unknown name id; a registered display name written literally in a text line
  (whole-word, case-sensitive match against every table value, so a rename
  can't leave stale text behind); the 200-char text limit measured with the
  longest display name in the table substituted for every token.
- Document the table and the token in `assets/dialogue/README.md`, and add a
  line to `docs/story/names.md`'s "How renaming works" pointing at
  `assets/data/names.ron`.

**Out (do not do):**
- `{lead}` and pronoun tokens, and the lead's name entry (0708 / 0801).
- Renaming anything. Nick decides names later; this ticket only makes it
  cheap.
- Replacing the placeholder cast in `characters.ron` with the real cast.
  That's the Chapter 1 content ticket (0803). Test characters may keep test
  names, as long as they come from the table.

## Implementation steps

1. `assets/data/names.ron`: a map `{ "retainer": "Hollis Marr", "king":
   "Emeric", "place.brennmark": "Brennmark", … }` for every row in
   `docs/story/names.md` (the lead's default first name `Ellery` goes in as
   `lead`), plus the test characters' names. Load it in `trpg_content`
   (`names.rs`, `Content::names`), and add it to the all-assets test.
2. Token syntax: `{n:<id>}`. Ids may contain `.` and `_`. It doesn't collide
   with 0708's `{lead}`/`{they}` tokens, which have no `n:` prefix. If 0708
   has already landed, reuse its token scanner.
3. `characters.ron`: replace `name: "…"` with the table lookup (for example,
   the character's `id` is its name id; `name` is removed or becomes
   optional, and must be absent for story characters). Update the loader and
   the error for a character with no table entry.
4. Validator additions in `crates/content/src/dialogue/check.rs` (see Scope).
   The literal-name check skips `#` comments and speaker ids.
5. Substitution in the dialogue player; the stored `Scene` keeps the raw
   token.
6. Docs: the format section in `assets/dialogue/README.md`, the
   `story-writing` skill's "Names are variables" section (point it at the
   token), and `docs/story/names.md`.

## Acceptance criteria

- [x] `names.ron` holds every id in `docs/story/names.md` (a test compares the id sets by parsing the Markdown table's first column).
- [x] Changing one entry in the table changes every line that uses its token (test with a scene using `{n:king}` twice).
- [x] Every new validator error has a test with its exact message.
- [x] Named characters' display names come from the table (test: a character's `Unit` name equals the table value).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: token parsing (valid, unknown id, unterminated `{n:`); the literal-name check (hit, miss, whole-word only); length check with substitution.
- Property: for random scenes with tokens, substituting then re-tokenising round-trips; the substituted text never contains `{n:`.
- Integration: the all-assets test loads `names.ron` and validates every `.dlg`.

## Completion notes

- `assets/data/names.ron` holds all 71 ids of `docs/story/names.md` plus the
  four placeholder test characters. It's loaded as `Content::names`
  (`trpg_content::names`) and validated: ids, duplicate ids, empty names,
  braces, non-ASCII, and two ids sharing one name. A test parses
  `names.md`'s tables and compares the id sets both ways.
- `characters.ron` no longer has `name:`; a named character's display name
  is the names-table entry for its id (a literal `name:` is now an unknown
  field). A character with no entry is an error that says what line to add.
  The dialogue name plate and the battle `Unit` name both come from it.
- `{n:<id>}` in speech, narration, captions and reply text, filled in by
  `DialoguePlayer::current()` (names first, then the lead's tokens), so
  stored scenes keep their tokens. `DialoguePlayer::new` and
  `DialogueScreen::new`/`overlay` now take the `Names`.
- Validator errors, each tested with its exact message: unknown name id;
  `{n:lead}` (write `{lead}`); a display name written out (also in
  captions and reply text); length with every name token counted as the
  longest name in the table (also applied to the lead's 40-character line
  and 60-character replies, for the same reason).

Deviations (technical, my call):
- Added `{N:<id>}`, the capitalised form, like 0708's `{They}`. Names keep
  their article ("the Thornmarch"), so without it a sentence couldn't start
  with one.
- The literal-name check strips a leading "the"/"a"/"an" (so
  "Thornmarch" alone is caught) and skips names with no capital letter
  ("breath", "a vow", "mor"): they are ordinary words and would flag normal
  prose.
- Where `names.md`'s "Current name" column holds more than a name, the
  table uses: `lead` "Ellery", `red_captain` "Harl Coster",
  `faction.brennmark` "the Brennish", gods in full ("Ama, the Mother").

Follow-up: **0712** short name forms (first names like "Hollis", god names
without titles). Each id has one name, and dialogue mostly says first
names, so the Chapter 1 script (0707) now waits for it; ROADMAP updated.

No gameplay rules were decided. Nothing visible changes in the game: the
test scenes don't use names yet.
