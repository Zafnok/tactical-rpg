---
id: "0712"
title: "Short name forms (first names) in the names table, so scripts can say \"Hollis\""
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0709"]
nick_input: none
completed:
---

# 0712 — Short name forms in the names table

## Context

0709 added `assets/data/names.ron` and name tokens (`{n:retainer}`) so every
story name can be renamed in one place (`docs/story/names.md`,
`assets/dialogue/README.md` "Names"). Each id has **one** display name, and
for most characters it is the full name: `retainer` is "Hollis Marr",
`sergeant` "Tamsin Rook", `rival` "Dace Marr". In dialogue people mostly say
the first name ("Hollis, wait."), and there is no token for it. The 0709
literal-name check only catches whole display names, so a script that writes
"Hollis" out slips through and a rename of Hollis Marr would leave it
behind. Gods have the same gap: `god.mother` is "Ama, the Mother"; scripts
will say "Ama" or "the Mother".

The Chapter 1 script (0707) needs this, so this ticket blocks it.

## Nick input

None. (Which short form each name has follows from the current names in
`docs/story/names.md`: the first word of a person's name; the name before
the comma for gods. The family names Marr and Veyne are shared, so they
get their own ids.)

## Scope

**In:**
- New ids in `docs/story/names.md` and `assets/data/names.ron` for short
  forms: `<id>.first` for every character whose name has two words
  (`retainer.first` → "Hollis", `rival.first` → "Dace", `sister.first` →
  "Wren", …), `<god id>.name` and `<god id>.title` for gods ("Ama", "the
  Mother"), `family.marr` → "Marr", `family.veyne` → "Veyne",
  `red_captain.nickname` → "Red Harl". Add them as rows (or a column) in
  `docs/story/names.md` so the id-set test (`names::tests::
  table_holds_every_registered_id`) still compares both ways.
- Because the short forms are table values, the existing literal-name check
  then catches "Hollis" written out. Check the existing `.dlg` files still
  pass.
- `docs/story/names.md` "How renaming works": renaming a character means
  changing both the full name and its short forms.

**Out (do not do):**
- Renaming anything.
- Any new token syntax: short forms are ordinary ids (`{n:retainer.first}`).
- Writing Chapter 1 dialogue (0707).

## Implementation steps

1. Add the rows to `docs/story/names.md` (new "Short forms" table) with the
   current values.
2. Add the same entries to `assets/data/names.ron`.
3. Run the content tests; fix any `.dlg` line the literal check now flags by
   using the token.
4. Update the `story-writing` skill's "Names are variables" section with one
   line: first names are `{n:<id>.first}`.

## Acceptance criteria

- [ ] `names.ron` has a short-form entry for every two-word character name,
      every god's name and title, and the family names.
- [ ] A test: a `.dlg` line writing "Hollis" out is an error naming
      `{n:retainer.first}`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the literal-name check flags a first name (in
  `crates/content/src/dialogue/tests/names.rs`).
- Integration: the all-assets test.

## Completion notes

