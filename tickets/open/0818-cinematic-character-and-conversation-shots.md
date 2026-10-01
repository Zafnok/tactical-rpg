---
id: "0818"
title: "Cinematic shots: characters, and conversation snippets"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0036", "0817", "0704", "0711"]
nick_input: answer-first
completed:
---

# 0818 — Cinematic shots: characters and conversation snippets

## Context

Nick's intro cinematic goes "past some characters" and shows "brief
conversaiton (non spoiler segments) snippets" (ticket 0036). 0817 built the
cinematic player with map pans and the logo. This ticket adds the two shot
kinds that show people.

How characters are shown (faces or combat pictures, with or without names)
and whether snippets use the real dialogue screen are Nick's answers in
0036 (`docs/design/title-screen.md`, *Intro cinematic*). The steps below
assume the likely answers: **bought faces** sliding past, and snippets in
**the game's dialogue screen**. If 0036 decided otherwise, it has updated
this ticket; follow the design doc where the two differ.

Character art is bought and drawn as pixel overlays (0711, ADR-0032).
Claude never draws character art.

## Nick input

**Answer first:** ticket 0036.

## Scope

**In:**
- Shot kind `Characters`: the listed characters' pictures move across the
  screen as the storyboard describes.
- Shot kind `Talk`: lines of a `.dlg` scene play by themselves in the
  dialogue screen's look (portraits, name plates, text box, typewriter
  text), with no input.
- Validator rules for both; both added to `assets/cinematics/README.md`
  and to `assets/cinematics/test.ron`.

**Out (do not do):**
- Choosing the characters or writing the lines (0820, from 0036's list).
- The title screen (0819).
- Combat pictures, unless 0036 chose them: then this ticket is also
  blocked by 0413 (0036 adds it) and reuses its drawing.

## Implementation steps

1. **`Talk(scene: "<id>", first: 0, count: 2)`**: text boxes `first ..
   first + count` of a dialogue scene. From the shot's `progress` and
   length, give each box an equal share of the time: reveal its text at
   `Ctx::text_speed` (capped so the whole box is shown for at least the
   last third of its share), then hold. (*Claude's starting values,
   tunable*, unless 0036's storyboard gives its own; list them in the PR
   for Nick.) Drawn from `t` alone (0817's rule), so split the drawing in `crates/ui/src/screens/dialogue.rs` into
   a function that takes the scene state (who is left and right, speaker,
   caption, text, characters revealed) and call it from both the screen
   and the shot. No "next" marker and no help line in the shot. Name and
   pronoun tokens (0708, 0709) resolve as in the dialogue screen.
   Validator: the scene exists; the range is inside it; no `@choice` in
   the range; every box fits one page (no paging in a cinematic).
2. **`Characters(who: ["<character id>", …], expression: "neutral")`**:
   each face is drawn with 0711's portrait drawing at its normal size and
   crosses the screen in turn, evenly spaced over the shot, in the
   direction and at the height the storyboard gives. Clipped at the screen
   edges. If the storyboard shows names, print the character's short name
   (0712) under the face. Validator: every character has a portrait with
   that expression.
3. Add one of each to `test.ron` using the test portraits and the `test`
   scene, and update the 0817 snapshots.

## Acceptance criteria

- [ ] Unit: the `Talk` timing (which box, how many characters revealed) at
      the start, mid-reveal, during the hold and at each box change.
- [ ] Snapshot: a `Talk` shot mid-reveal and held; a `Characters` shot with
      one face part-way in and one centred.
- [ ] The dialogue screen's own snapshots are unchanged.
- [ ] Each validator error has a test.
- [ ] What is drawn matches 0036's storyboard rules (list in the completion
      notes which rules were applied).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: validator; `Talk` timing.
- Property: for any `t` in the shot, the revealed length never exceeds the
  box's text and never shrinks as `t` grows within one box.
- Snapshot / integration: the snapshots above; the debug "Play test
  cinematic" tool still loops.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
