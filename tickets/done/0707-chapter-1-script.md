---
id: "0707"
title: Chapter 1 script (all scenes as .dlg)
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: done
blocked_by: ["0701", "0702", "0009", "0708", "0709", "0712", "0714"]
nick_input: sign-off
completed: 2026-10-02
---

# 0707 — Chapter 1 script

## Context

Steps 5–7 of the story pipeline
([ADR-0011](../../docs/adr/0011-story-authoring-pipeline.md)): turn the
Chapter 1 beat sheet (`docs/story/chapters/ch01.md`) into `.dlg` scenes.
Follow the `story-writing` skill, including the **mandatory critique pass**.

**Blocker added 2026-10-02:** 0714. The script check rejects a speaker who
isn't in `assets/data/characters.ron`, and the real cast wasn't in it (0803
adds them as units, and 0803 waits for this ticket). 0714 lets the cast
speak before they are units.

**Portraits (0706): either order works.** A script line names a character
and an expression, never a picture file; the game finds the face by
character id when it draws the box, and a character with no face yet gets
an empty frame with a name plate. Whether or not 0706 is done, follow the
expression rule in step 2.

## Nick input

**Sign-off:** Nick reads it in-game (after 0801/0803 wire it up) or as text
now, and comments. Offer him the text version in the PR description summary.

## Scope

**In:** `assets/dialogue/ch01.dlg` (or several files): opening/intro, pre-battle,
in-battle lines (boss engage, turn events; `chapter-1.md` has no talk-recruit),
death quotes for every Chapter 1 playable character and the boss, victory
scene, "to be continued" tease. Ledger update.

**Out:** trigger wiring (0803 wires scene ids into chapter data), portraits
(0706), the sheets' extra expressions (`weary`, `sneer`, `cold`, `smug`,
`flustered`, `determined`; see step 2), classes or stats for anyone (0803).

## Implementation steps

1. Reread the canon (skill's canon order) and the sheets of every character in Chapter 1.
2. Write scenes following the beat sheet and the lead rules in
   `docs/design/setting-and-tone.md`: the lead is gender-neutral (`{lead}` and
   pronoun tokens, 0708), speaks only through `@choice` blocks and short
   neutral lines, and gets 1–3 choice points in the chapter. No romance. Budget: opening + pre-battle ≤ ~40
   text boxes total (players want to play); in-battle lines 1–4 boxes each;
   victory ≤ ~20 boxes.
   - **Expressions:** use only `neutral`, `happy`, `angry`, `sad` and
     `surprised`, for everyone. The check allows only these five for a
     character without a portrait, the lead's two placeholder portraits
     have only these five, and 0706 must map these five for every bought
     face. The sheets' extras fail the check (the bought packs may not
     have them at all), so where the beat sheet says `weary` or
     `flustered`, pick the closest of the five and carry the rest in the
     words.
   - **Speakers:** the eight cast ids in `ch01.md`, *Cast on screen*, and
     `lead` can speak (0714). If a scene needs anyone else (the Vigil
     messenger in `ch01_tbc`, a Red Company soldier), add them in this
     ticket: an id in `speakers` in `assets/data/characters.ron`, and a
     name in `docs/story/names.md` and `assets/data/names.ron` (a test
     keeps those two in step). A Red Company soldier's id is `soldier`, so
     it gets 0706's shared `soldier` portrait.
3. Scene ids: `ch01_intro`, `ch01_prebattle`, `ch01_boss_engage`,
   `ch01_death_<char>`, `ch01_victory`, `ch01_tbc`
   (list them at the top of the file in a comment for 0803).
4. Validate with the content test (speakers, expressions, lengths).
5. **Critique pass** per the skill's checklist; revise.
6. Update `docs/story/ledger.md`.

## Acceptance criteria

- [x] All scenes validate in the all-assets test.
- [x] Critique checklist completed (paste the checked list in Completion notes).
- [x] Every Chapter 1 beat covered; no contradictions with `beats.md` / bible.
- [x] Ledger updated.

## Completion notes

Worked by Opus 5.5, not Fable 5.1 as routed (Nick's choice for this
session).

**What was written:** `assets/dialogue/ch01.dlg`, 21 scenes. Its header
comment lists every scene id with the trigger it is written for (0803
wires them).

| Scenes | Text boxes | Budget |
| ------ | ---------- | ------ |
| `ch01_intro` + `ch01_prebattle` | 38 or 39 (by reply) | about 40 |
| `ch01_first_turn` (Hollis) | 1 | 1–4 |
| `ch01_boss_engage`, `_lead`, `_sergeant`, `ch01_boss_half` | 2, 3, 4, 2 | 1–4 each |
| `ch01_death_red_captain` | 3 | 1–4 |
| `ch01_death_lead` | 1 | 2 |
| `ch01_death_<id>`, `ch01_retreat_<id>` for the five companions | 2 each | 2 |
| `ch01_victory` | 20 | about 20 |
| `ch01_tbc` | 6 | 6 |

Reply choices: three (the letter, the plan, leaving), each earnest / wry /
blunt. Only the five standard expressions are used.

**Story choices made here** (the beat sheet left them to this ticket, or
the script needed them; Nick can overrule any):

1. **A Vigil messenger, not Crane, in the last scene.** Crane is named
   only by his office ("the Master of Vows"), so his first appearance
   stays Chapter 7's.
2. **The letter is unsigned** (the beat sheet signed it "-D."). Hollis
   knows the hand, and the lead says "Dace." It makes the beat "Hollis
   recognises his son's writing" do work, and "Burn this." ends the letter.
3. **Hollis doesn't burn the letter.** He keeps it in his coat. Dace, in
   the last scene, burns his report.
4. **Dace's last line:** "I told you to run, Veyne."
5. **Rue says why she comes along:** "I'm not following you, heir. I'm
   walking behind you. Where I can see your hands."
6. **Harl answers the lead's "Who's paying?" with "Ask me when I'm dead"**,
   and does, in his death quote.
7. **Tamsin's line to Harl mentions "the farm"** and nothing more. Coldwell
   is still hers to tell later.
8. **Aske's reason is three deer** the Red Company killed and left to rot
   (the sheet's sample said spring; it's autumn).

**Added beyond the ticket's list of scenes:**

- A **Casual retreat line** for each companion (`ch01_retreat_<id>`), next
  to the Classic death quote. `death-and-difficulty.md` gives Casual a
  retreat line in the same trigger slot; a dying speech before a unit comes
  back next battle would read wrong.
- **Harl's half-HP line** (`ch01_boss_half`): every boss has one
  (`battle-scenes-and-recruitment.md`).
- **Hollis's first-turn line** (`ch01_first_turn`), optional in the beat
  sheet.

**Names and speakers added** (`docs/story/names.md`, `names.ron`):
`sergeant.last` (Rook; Harl's line needs it, and 0713 adds the other
surnames), `vowmaster.title` (the Master of Vows), `faction.brennmark.adj`
(Brennish, without the article: Tamsin's nickname for Aske), and
`messenger` (Messenger), who is also a new speaker in `characters.ron`. No
`soldier` speaker: no Red Company soldier speaks.

**Other files:** the beat sheet (`chapters/ch01.md`) notes each choice
above where it left one open; `characters/rival.md` (he answers Crane's
messenger); `assets/dialogue/README.md` (the new short forms); the ledger's
"After Chapter 1".

**Not solved here: a companion who died in Classic still talks in
`ch01_victory`.** A scene is one fixed list of lines, so the script can't
leave anyone out. The scene is written for a battle everyone lived
through. Follow-ups: **0715** (script lines that depend on who is still in
the army) and **0716** (the Chapter 1 scenes rewritten with it). Both were
added to 0804's `blocked_by` and the roadmap's critical path, since Nick
would see it in the playtest if he loses someone in Classic; he hasn't
been asked whether the playtest should wait for them. The death and
retreat quotes were written so none speaks to another companion by name
(they can fire in any order).

**Notes for other tickets** (edited): 0803 step 4 points at the scene list
and at Aske's count ("twelve men, no, fourteen", and the reply "Only
fourteen?"), so the map's enemy count should fit; 0713 knows
`sergeant.last` exists.

**Critique pass.** Done as a separate reviewer session that read the canon
and the script without my notes, plus my own read. It found 14 things;
all but two were fixed:

- The wry reply in `ch01_victory` never said where they were going: the
  plan is now in Hollis's line before the choice.
- "Dace is Hollis's son" didn't land in the intro: Hollis now says "Not to
  his own father."
- Tamsin was surprised by a crest Harl had just shouted about: "He wasn't
  lying, then."
- The last scene's report said "Nobody else is [dead]", false if a
  companion died: now "The exile is not."
- Harl's general fight line repeated his line to the lead and had no
  price in it: rewritten ("Three heads on my paper...").
- Piers's and Rue's death quotes spoke to each other, who may have fallen
  first: reworded.
- Rue had no stated reason to travel with the lead: added.
- One rhythm ("hard line. ...soft reversal") was shared by four voices,
  and Piers trailed off in every line: several lines recast.
- The lead's "What cart?" (the lead stood by Vosse's cart that night)
  became "Where?"; the lead's "Not... yet..." was cut.
- Smaller: "Figures." said by two people, "four years" in adjacent boxes,
  the ledger joke used five times, "noon" twice, the sash on a chair
  instead of on Dace, the ledger in a saddlebag instead of on her belt.
- Left as they are: Harl's "a hot meal for my lads" to the lead (the
  sheet's own line; it can play when his men are already dead), and one
  sword being shown where the beat sheet has two (Hollis fights with a
  spear).

The checklist:

- [x] Any line a different character could have said unchanged (voice
      failure). Two found and rewritten (Tamsin's last line to Harl; Rue's
      "Don't let it go to your head").
- [x] Exposition dumps, "as you know" dialogue. None; the longest run of
      explanation is three boxes (the orders, the crest, the title).
- [x] Contradictions with beats, bible, ledger. None found in timeline,
      ages, who holds the seal or what Harl knows. Nothing from Chapters
      3–7 is revealed. One fixed ("What cart?").
- [x] Scenes where nothing changes. None: every scene changes
      information, a relationship, the plan or the stakes; each in-battle
      line answers what the player just did.
- [x] Lines over length budget; unknown expressions/speakers. The
      validator passes; box counts are in the table above.
- [x] The chapter advances the main plot AND at least one personal arc.
      Main plot: the kill order under the Veyne seal, and the lead leaving
      exile to ask Dace. Arcs: Hollis (his son's hand, then his son's
      seal, and his silence), Tamsin (Harl), Piers (his report), Rue (the
      seal).

**Gameplay rules decided where the design was silent:** none. This ticket
writes text only.

