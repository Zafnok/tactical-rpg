---
id: "0041"
title: "Decide: the sound when a press skips the title cinematic"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: low
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0041 — Decide: the sound when a press skips the title cinematic

## Context

Ticket 0036 decided how the title's intro cinematic behaves
(`docs/design/title-screen.md`, *Intro cinematic*). Rule 4: a press during
the cinematic skips to the song's silent ending and shows the logo and
menu, "with some sound effect i.e. swords clashing" (Nick, 2026-10-02).

Which sound was not picked. Until it is, the game uses its sword hit
(`hit_sword`, `sfx/sword_sound_1.ogg`) as a stand-in (0819). That sound is
one blade landing; Nick asked for swords *clashing*.

The rules from 0020 apply (`docs/design/audio.md`): free sounds under CC0,
CC-BY 4.0 or CC-BY 3.0 (ADR-0013, ADR-0027), every sound credited, nothing
that costs money.

## Nick input

**Decision.** Nick listens to 4–6 candidates on a listening page and picks
one, or asks for another round.

## Questions

1. **Which sound** plays when a press skips the cinematic? Options: the
   candidates on the page (each with author, licence and a one-line
   description), **keep the sword hit we already have**, **one of our own
   made sounds** (name the closest from `audio.md`), or **describe your
   own** (then do another round).
2. Only if he raises it: whether the same sound should play anywhere else
   (for example when New Game is chosen). Don't offer it unasked.

## Scope

**In:**
- Find candidates (freesound, OpenGameArt, the libraries vetted in 0020):
  two blades meeting, short (under about 1.5 s), no long ring-out that
  would hang over the silence. Check each licence on its source page.
- A listening page (an Artifact, like 0020's and 0022's). Each candidate
  plays the way the game will: the title song playing, then the song cuts
  out and the sound plays over the logo and menu. Use the `ask-nick`
  skill.
- Record the answer in `docs/design/audio.md` (a row for cue `title_skip`
  with source and licence, Nick's words in the appendix) and replace the
  "skip sound" line under *Claude's starting rules* and *Open
  sub-questions* in `docs/design/title-screen.md`.
- If 0819 is done: import the file (`assets-src/audio/import.py`), add the
  cue `title_skip` to `assets/audio/audio.ron`, a row to
  `THIRD_PARTY_ASSETS.md`, the credit (0808 if it is done, else the
  manifest's credit entry is enough), and point 0819's constant at it.
  If 0819 isn't done: add a line to 0819's steps naming the chosen cue and
  file, and do the import there.

**Out (do not do):**
- Changing when the sound plays or what the press does (decided in 0036).
- Sounds inside the cinematic itself: it makes none
  (`title-screen.md`, *Claude's starting rules*).

## Implementation steps

1. Read `docs/design/audio.md` and `title-screen.md` (*Intro cinematic*).
2. Shortlist 4–6 sounds; note author, licence, length.
3. Build the listening page and run the question with `ask-nick`.
4. Record the answer and import or hand over as described in Scope.

## Acceptance criteria

- [ ] `audio.md` has a `title_skip` row with a source URL and a licence
      that ADR-0013/0027 allow, or a recorded "keep `hit_sword`".
- [ ] `title-screen.md` no longer lists the skip sound as open.
- [ ] Nick's words are quoted in `audio.md`'s appendix.
- [ ] If 0819 was done: the title plays the chosen cue on a skip (Harness
      test on the sound request) and content validation passes.
- [ ] `cargo xtask ticket-lint` passes; all gates in the `run-gates` skill
      pass if files under `assets/` or `crates/` changed.

## Tests required

- None for the decision. If the cue is imported: the existing audio
  manifest validation, and 0819's Harness test updated to the new cue id.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
