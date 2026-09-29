---
id: "0023"
title: "Decide: the defeat sting"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0023 — Decide: the defeat sting

## Context

Ticket 0020 picked the game's music ([`docs/design/audio.md`](../../docs/design/audio.md)),
but left some places and moments without a cue: "capital city, world map,
camp, shops, victory and defeat stings, game over, level up. Nick hasn't been
asked about these." (open sub-questions). Nick asked (2026-09-29) for each of
these moments to get its own ticket. Level up is not one of them: Nick is
handling the level-up and EXP sounds himself.

This ticket picks **the defeat sting**, cue id `defeat`. It plays when
a battle is lost (in Chapter 1: the lord falls, `death-and-difficulty.md`): it plays once while the `DEFEAT` banner is shown, before the Game Over screen.

Plays **once** (`looped: false`), a few seconds long.

Needed for Chapter 1: its implementation ticket 0809 blocks the playtest (0804).

The rules from 0020 still apply:

- Recorded music by other composers, under CC0, CC-BY 4.0 or CC-BY 3.0
  (audio.md rule 1, ADR-0013, ADR-0027). Nothing that costs money, nothing
  copyleft or non-commercial.
- Avoid composers who register their tracks with YouTube Content ID (0020
  left out Scott Buckley, Alexander Nakarada and Vindsvept for this reason;
  see 0904).
- Prefer looping and no-vocal versions (rule 9); every track is credited
  (rule 3).
- Music comes in tiers (rule 4): nothing from the epic tier here unless Nick
  says so.

## Nick input

**Decision.** Nick listens to 4–6 candidate tracks on a listening page and
picks one, or asks for another round.

## Questions

1. **Which track** plays for `defeat`? Options: the candidates on the
   page (each with composer, licence and a one-line description), **reuse a
   cue we already have** (name the closest ones from audio.md's table),
   **no music here**, or **describe your own** (then do another round).
- Or no separate defeat sting: go straight to the game-over music (0024)?

## Scope

**In:**
- Find candidates (OpenGameArt, freesound, itch.io free assets, and the
  libraries vetted in 0020). Check each licence on its source page.
- A listening page (an Artifact, like 0020's) with players for every
  candidate and the question above. Use the `ask-nick` skill.
- Record the answer in `docs/design/audio.md`: a row in *Music cues* (cue,
  when it plays, track, composer, source, licence, version to import), Nick's
  words in the appendix, and remove the moment from *Open sub-questions*.

**Out (do not do):**
- Importing or playing the file. Ticket 0809 does that (import with
  `assets-src/audio/import.py`, then `assets/audio/audio.ron`,
  `THIRD_PARTY_ASSETS.md` and the credits).
- Level-up or EXP sounds (Nick's own work).
- Other moments: each has its own ticket (0022–0028).

## Implementation steps

1. Read `docs/design/audio.md` and 0020's appendix for the tone Nick liked.
2. Shortlist 4–6 tracks that fit the moment and the licence rules. Note
   version (loop / no vocal) and length.
3. Build the listening page and run the question with `ask-nick`.
4. Record the answer in `audio.md` as above.

## Acceptance criteria

- [ ] `audio.md` has a `defeat` row with a source URL and a licence that
      ADR-0013/0027 allow, or a recorded "no music here" / "reuse `<cue>`".
- [ ] The moment is gone from audio.md's *Open sub-questions*.
- [ ] Nick's words are quoted in audio.md's appendix.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (design decision; docs only).

## Completion notes

*(Filled in by the session that completes the ticket.)*
