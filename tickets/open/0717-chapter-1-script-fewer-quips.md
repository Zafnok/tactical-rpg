---
id: "0717"
title: "Chapter 1 script: play the opening and the battle straight (fewer quips)"
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0707"]
nick_input: sign-off
completed:
---

# 0717 — Chapter 1 script: play the opening and the battle straight

## Context

Nick read lines from the Chapter 1 script (`assets/dialogue/ch01.dlg`,
ticket 0707) in the title cinematic mockups (ticket 0036) and rejected the
tone:

> dawg these dialogue lines are cheesy and cringe asf we'll need to rewrite
> them.
>
> there's just too many quips in general as a starting tone... quips should
> be during downtime, not on ch1 opening not on a serious battle... I'm not
> against quips as support ranks rise as the game reaches more high notes
> in terms of closeness but this just ain't it chief.

The lines he saw were Hollis's "That eyebrow's had one scar off me already,
and I'm not proud of it." / "A bit proud.", Aske's "Twelve men. No.
Fourteen. Bad archers.", and Rue's "I'm not a cook, Keeper. I'm a girl who
can light the oven without a match. Different thing." They are typical of
the script: nearly every character answers danger with a joke.

The rule is now in `docs/design/setting-and-tone.md` (*Where the jokes
go*): the game's opening and serious battles are played straight; banter
belongs to downtime and grows as characters get closer (support ranks).
The tone is still "dark with warmth and humour"; this changes **where**
the humour sits, not whether the game has any.

The title cinematic plays the opening lines of `ch01_intro` and
`ch01_prebattle` straight from this file (`title-screen.md`, *Intro
cinematic*), so what this ticket writes is also what the title shows.

## Nick input

**Sign-off:** the PR lists, scene by scene, the lines that changed (old
and new, as text). Nick reads them and comments. Claude writes every line
(Nick writes no dialogue); only tone and story decisions go to him.

Whether this must land before his Chapter 1 playtest (0804) is his call.
He was asked when this ticket was written (2026-10-03); if 0804's
`blocked_by` doesn't list this ticket and nothing here records a "no",
ask him once before starting, and add it to 0804 and the roadmap's
critical path on a yes.

## Scope

**In:**
- Rewrite `assets/dialogue/ch01.dlg` with the `story-writing` skill so
  that:
  - `ch01_intro` and `ch01_prebattle` are played straight: people who have
    just learned that armed men are coming for them talk like it. Warmth
    (Hollis's care for the lead, Piers's kindness) stays; one-liners,
    running gags and comic bits go, or move to the wry reply choice.
  - Battle scenes (`ch01_first_turn`, the boss scenes, death and retreat
    lines) carry no jokes. A dry or bitter line that is in character
    (Harl's "Still owed.") is fine; a punchline is not.
  - `ch01_victory` and `ch01_tbc` stay serious; at most one light moment
    after the tension breaks.
- Characters keep their voices (`docs/story/characters/*.md`): a voice is
  how someone talks, not how often they joke. Where a sheet's voice notes
  or sample lines push a character toward constant quipping, update the
  sheet too.
- **The lead's `wry` reply choices stay** (`setting-and-tone.md` rule 2:
  replies come in distinct tones and the player picks). Humour the player
  chose is not the script quipping at them. The other characters' answers
  to a wry reply may be light, briefly.
- Every story fact and every scene id stays: what the letter says, who is
  on Harl's paper, the seal, the count of fourteen, who joins and why. The
  ledger (`docs/story/ledger.md`) changes only where a rewritten line
  stated a fact differently.
- A short note in the `story-writing` skill (its tone section) pointing at
  *Where the jokes go*, so later chapters start from this rule.

**Out (do not do):**
- New scenes, new plot, new characters, changed outcomes.
- The scenes for a dead companion (0716). If 0716 is done, rewrite its
  lines to the same rule in this ticket; if it isn't, add a line to 0716
  pointing at *Where the jokes go* so it is written straight from the
  start.
- Support conversations and camp banter (1002, 1003): that is where the
  banter goes later; nothing to write now.
- The title cinematic (0820): it reads this file and needs no change.

## Implementation steps

1. Read `setting-and-tone.md` (all of Q2 and *Where the jokes go*), the
   `story-writing` skill, `docs/story/chapters/ch01.md`, the character
   sheets and the ledger.
2. Go through `ch01.dlg` scene by scene. For each joke, decide: cut, turn
   into a plain line that carries the same information, or keep (a wry
   reply's answer, or the single light moment in the victory scene). Keep
   a list of old and new lines as you go: it is the PR's sign-off text.
3. Keep every line inside the dialogue screen's limits (three rows per
   box; the content validator checks). Keep `@choice` blocks, name tokens
   and scene ids as they are.
4. Update character sheets, the beat sheet and the ledger where they quote
   or depend on a changed line.
5. Run content validation and the dialogue tests; update snapshots that
   show changed text.

## Acceptance criteria

- [ ] `ch01_intro`, `ch01_prebattle` and every battle scene contain no
      joke by a character other than in answer to the lead's `wry` reply;
      the PR's list shows each removed or changed line.
- [ ] Every scene id, `@choice` and story fact of 0707 is still there
      (the ledger's Chapter 1 entries still hold).
- [ ] Content validation passes for `ch01.dlg`.
- [ ] The `story-writing` skill points at *Where the jokes go*.
- [ ] If 0716 was done, its scenes follow the rule too; if not, 0716 says
      to.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: existing dialogue and flow tests that show
  Chapter 1 text, updated; content validation of the script.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
