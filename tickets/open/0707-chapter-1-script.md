---
id: "0707"
title: Chapter 1 script (all scenes as .dlg)
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0701", "0702", "0009", "0708"]
nick_input: sign-off
completed:
---

# 0707 — Chapter 1 script

## Context

Steps 5–7 of the story pipeline
([ADR-0011](../../docs/adr/0011-story-authoring-pipeline.md)): turn the
Chapter 1 beat sheet (`docs/story/chapters/ch01.md`) into `.dlg` scenes.
Follow the `story-writing` skill, including the **mandatory critique pass**.

## Nick input

**Sign-off:** Nick reads it in-game (after 0801/0803 wire it up) or as text
now, and comments. Offer him the text version in the PR description summary.

## Scope

**In:** `assets/dialogue/ch01.dlg` (or several files): opening/intro, pre-battle,
in-battle lines (boss engage, turn events; `chapter-1.md` has no talk-recruit),
death quotes for every Chapter 1 playable character and the boss, victory
scene, "to be continued" tease. Ledger update.

**Out:** trigger wiring (0803 wires scene ids into chapter data), portraits (0706).

## Implementation steps

1. Reread the canon (skill's canon order) and the sheets of every character in Chapter 1.
2. Write scenes following the beat sheet and the lead rules in
   `docs/design/setting-and-tone.md`: the lead is gender-neutral (`{lead}` and
   pronoun tokens, 0708), speaks only through `@choice` blocks and short
   neutral lines, and gets 1–3 choice points in the chapter. No romance. Budget: opening + pre-battle ≤ ~40
   text boxes total (players want to play); in-battle lines 1–4 boxes each;
   victory ≤ ~20 boxes.
3. Scene ids: `ch01_intro`, `ch01_prebattle`, `ch01_boss_engage`,
   `ch01_death_<char>`, `ch01_victory`, `ch01_tbc`
   (list them at the top of the file in a comment for 0803).
4. Validate with the content test (speakers, expressions, lengths).
5. **Critique pass** per the skill's checklist; revise.
6. Update `docs/story/ledger.md`.

## Acceptance criteria

- [ ] All scenes validate in the all-assets test.
- [ ] Critique checklist completed (paste the checked list in Completion notes).
- [ ] Every Chapter 1 beat covered; no contradictions with `beats.md` / bible.
- [ ] Ledger updated.

## Completion notes

