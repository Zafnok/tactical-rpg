---
id: "0717"
title: Every dialogue line gets a stable id
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-03
---

# 0717 — Every dialogue line gets a stable id

## Context

Translations (ADR-0045) and voice clips (ADR-0046) both attach to single
dialogue lines, and a rewritten line must lose both. Lines have no ids
today; scenes do. [ADR-0045 §3](../../docs/adr/0045-languages-text-by-key-and-line-ids.md)
decides the id: `<scene id>_<8 hex>`, a hash of the speaker and the English
text as written, computed by the loader, never written in the script.

## Nick input

None.

## Scope

**In:** `LineId` in `trpg_content::dialogue`; an id on every speech line,
narration line and reply; a listing command.

**Out (do not do):** translation files (0235); voice (0238); any change
to the `.dlg` syntax or to what the dialogue screen shows. Ticket 0715
(lines that depend on who is still in the army) changes the parser too:
either order works; whichever lands second gives its new line kinds ids
the same way.

## Implementation steps

1. `crates/content/src/dialogue.rs`: add `pub struct LineId(String)`
   (ordered, hashable, `Display`). Add a `line: LineId` field to the
   `Step` variants that show text (speech, narration) and to
   `ChoiceOption` (the reply text). Lines inside a reply's reaction get
   ids like any other.
2. Compute it in `dialogue/parse.rs` once a scene is complete:
   `fnv1a64(speaker_id + "\n" + text)` where `text` is the joined line as
   written (tokens unexpanded, continuation lines joined with one space),
   and `speaker_id` is the character id, `>` for narration, `*` for a
   reply. Take the low 32 bits as 8 lowercase hex digits. Write FNV-1a
   in the module (a dozen lines); don't use `std::hash`, whose output may
   change between Rust versions.
3. A repeat of the same speaker and text in one scene gets `_2`, `_3`…
   in script order. Ids are unique across all scenes because scene ids
   are; assert it in `dialogue/check.rs` (a hash collision between two
   different lines of one scene is an error naming both lines; the fix is
   to reword one).
4. `cargo xtask lines [scene]`: prints `id<TAB>speaker<TAB>text` for
   every line, for the tools that come later and for humans.
5. Document ids in `assets/dialogue/README.md`: what they are, that
   rewording a line changes its id, and that this is what marks its
   translation and voice as out of date.
6. Set ADR-0045's status to `Accepted` if 0233 has landed; otherwise
   leave it and say so in the completion notes. Record any change to §3.

## Acceptance criteria

- [x] Every text-showing step and reply in every `.dlg` has an id; ids are unique (all-assets test).
- [x] The id of a known line is a fixed string in a test (so the hash can never change silently).
- [x] Inserting a line above another, or moving a line within its scene, leaves the other ids unchanged (test).
- [x] Changing one word changes that line's id only (test).
- [x] Two identical lines in a scene get `_2` on the second (test).
- [x] `cargo xtask lines ch01_intro` prints one row per line.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the hash against a hand-computed value; repeats; reply and reaction lines.
- Property: for random scenes, inserting a line never changes another line's id.
- Integration: the all-assets test checks uniqueness.

## Completion notes

- `LineId` and the hash are in `crates/content/src/dialogue/line_id.rs`.
  `Step::Say`, `Step::Narrate` and `ChoiceOption` carry a `line: LineId`;
  the parser fills it when a scene is complete (`Scene::assign_line_ids`).
  `Scene::lines()` lists every line with its id, speaker and text, in
  script order (a reply, then its reaction, then the next reply).
- Ids are unique within a scene by construction (repeats are numbered per
  hash). `dialogue/check.rs` reports two *different* lines of one scene
  with the same hash, naming both; the test uses a real colliding pair
  found by search. The all-assets test checks every id's shape and that no
  two lines of any `.dlg` share one.
- `cargo xtask lines [scene]` prints `id<TAB>speaker<TAB>text`
  (`crates/xtask/src/lines.rs`); without a scene, every scene by scene id.
- A step built by hand (tests) has an empty id until `assign_line_ids`
  runs. Nothing on screen changed; `ui` ignores the ids.
- `assets/dialogue/README.md` has a "Line ids" section.
- **ADR-0045 stays `Proposed`**: ticket 0233 has not landed. §3 gained an
  "As built" paragraph (what stands for the speaker of narration and
  replies, the low 32 bits, expression and tone not hashed, the clash
  error, the listing command). No decision in it changed.
- No deviations from the implementation steps.
- No gameplay rules were decided. No follow-up tickets.
