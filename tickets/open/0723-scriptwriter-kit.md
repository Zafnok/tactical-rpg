---
id: "0723"
title: "A scriptwriter's kit: guide, script checker and scene preview for someone who doesn't program"
type: infra
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0715", "0717"]
nick_input: none
completed:
---

# 0723 — A scriptwriter's kit: guide, script checker and scene preview for someone who doesn't program

## Context

Nick, 2026-10-03 (`docs/design/voices-languages-and-script.md`): the
script is written by Claude today; if the game finds interest he may hire
a scriptwriter, and "since our scripts are human readable/writable (I
hope) then it shouldn't be a huge hassle for them to also add cues for
i.e. music, portrait transitions, etc."

The `.dlg` format is plain text with those cues (`@music`, `@left`,
`@right`, `@caption`, `@choice`; `assets/dialogue/README.md`). What a
hired writer lacks today: the format's reference assumes the repo; the
only checker is `cargo test`; the only way to see a scene is to play to
it. 0715 changes the format (lines for who is still in the army), so
this waits for it.

## Nick input

None.

## Scope

**In:** a writer's guide; a one-command checker with messages a writer
can act on; a way to watch one scene; a hand-over page listing what a
writer needs to read.

**Out (do not do):** changing the `.dlg` format; rewriting any scene; a
visual script editor; tooling for translators or actors (0235's
`lang-status`, 0721's recording scripts). Don't weaken a validator rule
to make writing easier: if a rule looks wrong for a human writer (plain
ASCII quotes, 200-character lines), list it in the notes for Nick.

## Implementation steps

1. `docs/story/writers-guide.md`, written for a professional writer who
   has never seen the repository: what a scene file is; every cue with a
   before/after example (music moods and what each cue sounds like, from
   `docs/design/audio.md`; portraits and each character's expressions);
   reply choices and the lead's rules (`docs/design/setting-and-tone.md`);
   name tokens and why names are never typed out; where scenes are
   wired to battles and chapters (`assets/chapters/`, `assets/battles/`,
   triggers per ADR-0030) and that Claude does that wiring on request;
   what canon is fixed (`docs/story/beats.md`) and what may change.
   Link, don't copy, `assets/dialogue/README.md` for the full reference.
2. Say in the guide what a rewrite costs downstream: a changed line
   loses its translation and its voice clip until they are redone
   (ADR-0045 §3, ADR-0046), and how to see the list.
3. `cargo xtask check-script [file]`: runs the dialogue validation alone
   (no test build), prints `file:line: message` with the fix suggested
   where the validator knows it (the ASCII form of a curly quote, the
   token for a typed-out name), and ends with a count. Reuse
   `trpg_content::dialogue::from_sources`; no new rules.
4. Scene preview: `cargo run -p trpg-app -- --scene <id>` opens the game
   straight on that scene (with the lead's default name; `--lead f` for
   the other gender) and returns to it on a key when it ends. If a debug
   scene viewer already exists in `crates/ui/src/debug`, make the flag
   open that instead. Native only.
5. A released build can't run `cargo`. Note in the guide the two ways a
   writer can work: with the repository and Rust installed (steps
   listed, Windows and macOS), or by sending files to Nick, who runs the
   checker. Don't build a standalone checker now; write a ticket for one
   only when a writer is actually hired.
6. `docs/story/README.md`: a "For a human writer" section linking the
   guide, bible, character sheets, outline and ledger in reading order,
   and stating that the ledger must be updated after a rewrite.

## Acceptance criteria

- [ ] `cargo xtask check-script` on the repository's scripts reports 0 problems; on a file with a curly quote and a typed-out name it reports both with line numbers and fixes (tests).
- [ ] `--scene ch01_intro` opens that scene (Harness test of the start-up path; manual check noted).
- [ ] The guide's every example passes `check-script` (a test extracts and checks the fenced examples).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `check-script` output formatting.
- Integration: the guide's examples; starting on a scene.

## Completion notes

