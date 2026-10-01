---
id: "0713"
title: "Surname short forms in the names table, so scripts can say \"Sergeant Rook\""
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: low
status: todo
blocked_by: ["0712"]
nick_input: none
completed:
---

# 0713 — Surname short forms in the names table

## Context

0712 gave every two-word character name a first-name id (`retainer.first` →
"Hollis") and gave the two *shared* family names their own ids (`family.marr`,
`family.veyne`). The surnames only one character has got no id: Rook, Crane,
Coster, Vosse, Pellam, Parrow, Ravn, Holt, Mast. Scripts will say "Sergeant
Rook", "Master Crane" or just "Vosse", and the literal-name check
(`Names::literal_in`, `crates/content/src/names.rs`) only catches values in
the table, so those would be written out and a rename would leave them
behind. Found while working 0712; out of its scope.

Not a blocker for the Chapter 1 script (0707): a writer who needs one of
these before this ticket lands adds that one id first (`docs/story/names.md`,
"New names get an id here first").

## Nick input

None. The surname is the second word of the character's current name in
`docs/story/names.md`.

## Scope

**In:**
- `<id>.last` for every two-word character name whose surname is not shared:
  `sergeant.last` → "Rook", `vowmaster.last` → "Crane", `red_captain.last` →
  "Coster", `vosse.last` → "Vosse", `defector.last` → "Pellam",
  `prizefighter.last` → "Parrow", `shieldbearer.last` → "Ravn", `envoy.last`
  → "Holt", `battlemage.last` → "Mast". Rows in the "Short forms" table of
  `docs/story/names.md` and entries in `assets/data/names.ron`.
- The Marrs and the lead keep `family.marr` / `family.veyne` (values must be
  unique, so no `retainer.last`).
- `assets/dialogue/README.md` "Short forms" table: one row for `.last`.

**Out (do not do):**
- Renaming anything; new token syntax; writing dialogue.

## Implementation steps

1. Add the nine rows to the "Short forms" table in `docs/story/names.md`.
2. Add the same entries to `assets/data/names.ron`, in the short-forms group.
3. Extend `names::tests::every_two_word_name_has_short_forms`
   (`crates/content/src/names/tests.rs`): a person's surname is either
   `<id>.last` or the value of a `family.*` id.
4. Add the README row. `Crane`, `Rook`, `Holt` and `Mast` are also ordinary
   English words: add them to the README's note on short forms that are
   ordinary words, and to the by-eye list in `docs/story/names.md` "How
   renaming works".
5. Run the content tests; the existing `.dlg` files must still pass.

## Acceptance criteria

- [ ] `names.ron` has a `.last` entry for each of the nine surnames above.
- [ ] A test: a `.dlg` line writing "Rook" out is an error naming
      `{n:sergeant.last}`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the extended short-forms test; the literal-name check flags a
  surname (`crates/content/src/dialogue/tests/names.rs`).
- Integration: the all-assets test.

## Completion notes

