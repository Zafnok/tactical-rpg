---
id: "0714"
title: Dialogue speakers who aren't units; add the Chapter 1 cast as speakers
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0714 — Dialogue speakers who aren't units

## Context

The dialogue check rejects any speaker who isn't a character in
`assets/data/characters.ron` (ticket 0702; `Checker::known` in
`crates/content/src/dialogue/check.rs`). Today that file holds only `lead`
and the `test_*` characters. The real cast (0701) has names in
`assets/data/names.ron` (0709) but no entries in `characters.ron`.

That made a hidden loop, found 2026-10-02:

- The Chapter 1 script (0707) must pass the check, so it needs `retainer`,
  `sergeant` and the rest to be known.
- The cast is added to `characters.ron` by 0803, which gives them classes,
  stats and loadouts.
- 0803 is blocked by 0707.

A `characters.ron` entry is a full unit: class, level, base stats, talent,
weapon ranks, and a loadout it can fight with (the
`embedded_characters_load_and_make_units` test). Several speakers will
never be units in Chapter 1 (Dace, Crane, a messenger), and Crane's class
doesn't exist yet (`docs/story/characters/vowmaster.md`). Placeholder unit
entries would mean inventing stats, which is 0803's job
(`docs/design/chapter-1.md`, *Player roster*).

So this ticket adds a second kind of entry: a **speaker**. A speaker is an
id that may appear in dialogue and has a name, and nothing else. The
Chapter 1 cast go in as speakers now. 0803 later moves the ones who fight
into `characters` with their real numbers.

## Nick input

None.

## Scope

**In:**
- A `speakers` list in `characters.ron`, loaded into `CharacterTable`.
- The dialogue check accepts a speaker wherever it accepts a character.
- The dialogue screen shows a speaker's name on the name plate.
- The eight Chapter 1 cast ids added as speakers: `retainer`, `sergeant`,
  `poacher`, `keeper`, `heretic`, `red_captain`, `rival`, `vowmaster`.
- `assets/dialogue/README.md` and the comment at the top of
  `characters.ron` say what a speaker is.

**Out (do not do):**
- Classes, stats, talents or loadouts for the cast, or moving anyone into
  `characters` (0803).
- Any dialogue lines (0707).
- Speakers the script may add (a Vigil messenger, a Red Company soldier):
  0707 adds those if it uses them.
- Portraits (0706). A speaker without a portrait already draws as an empty
  frame with a name plate, and may use the five standard expressions.
- Letting a speaker be placed on a map or named in a battle or chapter
  file. Those keep requiring a `characters` entry.
- Removing the `test_*` characters (0803).

## Implementation steps

1. **`crates/content/src/character.rs`**
   - `RawFile`: add `#[serde(default)] speakers: Vec<String>`.
   - `CharacterTable`: add `pub speakers: BTreeSet<CharacterId>` with a doc
     comment ("Ids that may speak in dialogue but aren't units"), and a
     method `pub fn can_speak(&self, id: &CharacterId) -> bool` that is true
     for a key of `characters` or a member of `speakers`.
   - In `from_source`, after the characters loop, check each speaker and
     report every problem with `v.err(id, …)`:
     - not a valid id (reuse `is_id` in
       `crates/content/src/dialogue/parse.rs`, the rule for character ids
       in scripts: lowercase letters, digits and `_`):
       `speaker "<id>": ids are lowercase letters, digits and _`;
     - listed twice: `duplicate speaker id "<id>"`;
     - also in `characters`:
       `"<id>" is both a character and a speaker; a character can already speak, so remove it from speakers`;
     - no name: call the existing `Validator::name(id, &format!("speaker \"{id}\""))`
       so the message matches the one characters get.
   - `Validator::err` finds the line with `id: "<id>"`, which speakers don't
     have. Give it a fallback: if that isn't found, use the line of
     `"<id>"`.
2. **`crates/content/src/dialogue/check.rs`**: `Checker::known` returns
   `self.characters.is_none_or(|t| t.can_speak(id))`. Nothing else changes:
   expressions already work for an id without a portrait.
3. **`crates/ui/src/screens/dialogue.rs`**, `display_name`: for anyone but
   the lead, return `ctx.content.names.get(&portrait.character.0)`, falling
   back to the id. A character's name already is its names-table entry
   (`Validator::name`), so this gives the same text for characters and the
   right text for speakers. Update the doc comment.
4. **`assets/data/characters.ron`**
   - Add, after `generics`:
     ```ron
     // speakers: ids that may speak in dialogue (.dlg) but aren't units
     // (yet). Each needs a name in names.ron. 0803 moves the Chapter 1
     // party and the boss up into `characters` with real stats.
     speakers: [
         "retainer", "sergeant", "poacher", "keeper", "heretic",
         "red_captain", "rival", "vowmaster",
     ],
     ```
   - Add a `speakers:` paragraph to the comment block at the top, next to
     `characters:` and `generics:`.
   - Change nothing in `characters` or `generics`.
5. **`assets/dialogue/README.md`**, *Rules*: the **Characters** rule
   becomes "must exist in `assets/data/characters.ron`, as a character or
   in its `speakers` list (someone who talks but isn't a unit)".
6. Search for other places that decide whether an id is a known person and
   so should accept speakers:
   `grep -rn "characters.characters" crates --include=*.rs`. Battle, chapter,
   trigger and spell code look up units and must stay as they are. Change
   only lookups made for dialogue or name plates, and list what you
   changed in Completion notes.

## Acceptance criteria

- [ ] A `.dlg` scene whose speakers are `retainer` and `rival` passes
      `check_scene` with the embedded data (test).
- [ ] A scene with a speaker in neither list still fails with
      `unknown character "<id>"` (the existing `unknown_characters` test
      still passes).
- [ ] Each speaker problem in step 1 has a test that checks the message and
      the line number.
- [ ] The dialogue screen's name plate for `retainer` reads the names-table
      entry ("Hollis Marr" today), not `retainer` (test).
- [ ] Every Chapter 1 cast id in `docs/story/chapters/ch01.md`, *Cast on
      screen*, can speak (test, see below).
- [ ] `characters` and `generics` in `characters.ron` are unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/content/src/character.rs`): a file with speakers loads;
  one test per error in step 1; a file without `speakers` still loads.
- Unit (`crates/content/src/dialogue/tests.rs`): a speaker may be placed
  and may speak; an unknown id still fails.
- Unit (embedded data, next to `embedded_characters_load_and_make_units`):
  `can_speak` is true for `lead` and the eight cast ids. Write it with
  `can_speak`, not by listing `speakers`, so it still passes after 0803
  moves some of them into `characters`.
- UI (`crates/ui/src/screens/dialogue/tests.rs`): a scene with a speaker
  on screen shows the speaker's name on the name plate. This tests a look,
  so it may read cells.
- No property or snapshot tests needed. If an existing dialogue snapshot
  changes, something is wrong: the name text must be the same as before
  for characters.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
