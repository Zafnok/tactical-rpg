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

Nick's answers in 0036 (2026-10-03, `docs/design/title-screen.md`, *Intro
cinematic*):

- **Six bought faces slide past with their names**: the lead, Hollis,
  Tamsin, Aske, Piers, Rue.
- **Snippets play in the game's dialogue screen**, and they are the
  **opening lines of real Chapter 1 scenes**, read from the same script
  file the game plays (`assets/dialogue/ch01.dlg`). No separate cinematic
  script.
- **The lead follows the most recent save**: its name and its face. With
  no save, the default lead.

Character art is bought and drawn as sprite items (0711, ADR-0032,
ADR-0038).
Claude never draws character art.

## Nick input

**Answer first:** ticket 0036 (answered 2026-10-03).

## Scope

**In:**
- Shot kind `Characters`: the listed characters' pictures move across the
  screen as the storyboard describes.
- Shot kind `Talk`: lines of a `.dlg` scene play by themselves in the
  dialogue screen's look (portraits, name plates, text box, typewriter
  text), with no input.
- Both shots show the lead as the most recent save has them (below).
- Validator rules for both; both added to `assets/cinematics/README.md`
  and to `assets/cinematics/test.ron`.

**Out (do not do):**
- Choosing which scenes and how many boxes the title cinematic shows
  (0820).
- Writing or changing dialogue. The shots show whatever the script file
  says.
- The title screen (0819).
- Combat pictures (0036 didn't choose them).

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
   Narration boxes and `@caption` lines in the range show as they do in
   the dialogue screen.
   Validator: the scene exists; the range is inside it; no `@choice` in
   the range; every box fits one page (no paging in a cinematic).
2. **`Characters(who: ["<character id>", …], expression: "neutral")`**:
   each face is drawn with 0711's portrait drawing at its normal size and
   crosses the screen in turn, evenly spaced over the shot, in the
   direction the shot gives (the storyboard: right to left), vertically
   centred. Clipped at the screen edges. The character's short name (0712)
   is printed under each face (0036: names are shown). Validator: every character has a portrait with
   that expression.
3. **The lead from the most recent save.** `CinematicPlayer::new` takes
   an optional lead (name and which of the lead's two looks), which the
   caller reads from the save written last. Saves carry no time (only
   `app` reads a clock), so make `save::write` (`crates/ui/src/save.rs`)
   also store the key it wrote under a new `Storage` key `last_save`, and
   add `save::latest_lead(storage)`: the lead of the save `last_save`
   names; if that key is missing or the save can't be read, the suspend
   save's, else the lowest filled slot's, else `None`. Take the name and
   look from the save's campaign the way the dialogue screen does.
   `Talk` resolves the lead's name and pronoun tokens with it and both shots draw that look's portrait.
   With no save: the names table's default name, and the default look
   that 0706 records (until 0706 says, `lead_m`'s portrait id, as the
   dialogue screen falls back today). The debug tool passes `None`.
4. Add one of each to `test.ron` using the test portraits and the `test`
   scene, and update the 0817 snapshots.

## Acceptance criteria

- [ ] Unit: the `Talk` timing (which box, how many characters revealed) at
      the start, mid-reveal, during the hold and at each box change.
- [ ] Snapshot: a `Talk` shot mid-reveal and held; a `Characters` shot with
      one face part-way in and one centred.
- [ ] The dialogue screen's own snapshots are unchanged.
- [ ] Each validator error has a test.
- [ ] Harness: with a save whose lead was renamed and uses the other
      look, a `Talk` box that names the lead and a `Characters` shot show
      that name and that portrait; with no save they show the default.
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
