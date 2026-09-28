---
id: "0701"
title: Story bible, main cast sheets, act outline, Chapter 1 beat sheet
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: done
blocked_by: ["0004", "0007", "0008"]
nick_input: sign-off
completed: 2026-09-28
---

# 0701 — Story bible, cast, outline

## Context

Steps 2–4 of the story pipeline in
[ADR-0011](../../docs/adr/0011-story-authoring-pipeline.md). Turns Nick's beats
(`docs/story/beats.md`) into a world, a cast with personal arcs, and an
outline. Follow the `story-writing` skill strictly.

## Nick input

**Sign-off at two gates** (keep each to a one-page summary):
1. World + cast summary.
2. Act outline.

## Scope

**In:** `docs/story/bible.md`, `docs/story/characters/*.md`,
`docs/story/outline.md`, `docs/story/chapters/ch01.md`, `docs/story/ledger.md`.

**Out:** dialogue scripts (0707), portraits (0706), game data files.

## Implementation steps

1. Read: `beats.md`, `docs/design/setting-and-tone.md`, `magic.md`,
   `world-structure.md`, `supports.md` (if exists), `chapter-1.md` (if exists),
   `progression.md` (class names to use for characters).
2. **bible.md:** world premise (1 paragraph), geography (5–8 named places, one
   line each), history (the event the story grows from), factions (3–5, what each
   wants), themes (2–3), tone rules (what jokes are OK, how dark it gets), rules
   of magic/tech consistent with `magic.md`, glossary. **Nick deferred "what
   is magic in this world?" to this ticket** (0004 Q3): propose 2–3 options
   at gate 1 and record his pick. It must fit the mechanics (innate spells
   with uses per battle, black and white magic, fire/ice terrain effects,
   uncommon elementals).
   Hard constraints from `setting-and-tone.md` (0007): tone B (dark with
   warmth and humour); the ending is a resolved, hard-fought victory, never a
   tragedy (scripted losses along the way are allowed); **no romance or
   shipping** (pre-existing relationships only); the world has other
   continents (e.g. a wuxia-inspired Asian one) that the bible leaves room for;
   every main companion has an arc or story role.
3. **Cast:** 8–12 characters for Act 1: the lord/protagonist, 4–6 playable
   companions (covering the Chapter 1 roster classes), 1 antagonist with an
   understandable motive, 1–2 recurring secondary villains/rivals, 1–2 NPCs.
   Each `characters/<id>.md` follows the skill: role, class, want, need, flaw,
   secret/pressure, arc (start → end), relationships (who they clash/bond
   with and why), voice notes + 3 sample lines, **portrait brief** (silhouette,
   hair, clothing, colours — used by 0706), expression list (at least the five
   standard ones), **support partners** (the 3–6 other characters this one
   has a C/B/A support with, including the lead where it fits, and one line
   on what each support is about: friendship, rivalry, mentorship or family,
   never romance; see `supports.md`), and for spellcasters **1–2 personal signature spells**
   (name, element, one-line effect; `magic.md`: numbers are set by 0005 or a
   balance ticket, not here).
   **The lead's sheet is different** (Persona-style lead, see "The lead" in
   `setting-and-tone.md`): id `lead`, 18–25, disgraced/exiled noble,
   gender chosen by the player. Record background, situation and
   want/pressure, but **no fixed personality or voice**. Instead give 3 reply
   tones (e.g. earnest / wry / blunt) with one sample choice. Give a portrait
   brief for **both** `lead_m` and `lead_f` (same age, costume and colours).
4. **Gate 1:** Nick deferred several things to this gate (0007), so ask him
   first, with the `ask-nick` skill (options from games he likes + "describe
   your own"): (a) 2–3 options for **why the lead was disgraced or exiled** and
   the inciting incident; (b) **leading questions about the cast** (he asked
   for them: e.g. the mentor figure, the rival, what the antagonist wants);
   (c) **can the player rename the lead?** Then send Nick a one-page summary
   (premise, factions in a line each, cast in a line each). Revise until he approves. Record his feedback verbatim
   at the bottom of `bible.md`.
5. **outline.md:** Act 1 in detail (6–10 chapters: goal, conflict, turn, map
   idea, which personal arcs advance, recruits); Acts 2–3 as a paragraph each
   with the ending. Follow `world-structure.md` (0008): chapters 1–2 (maybe 3)
   are linear, single-battle chapters (propose whether Chapter 3 is linear at
   gate 2); after that, each world-map chapter names its story battle, fixed
   skirmishes (which guard paths), towns, and side quests (whose story, unlock
   condition). Each continent is its own act; say which acts change continent.
   No branching routes. Every chapter must advance the main plot **and** a personal arc.
6. **Gate 2:** one-page outline summary to Nick, including 2–3 options for the
   **midpoint twist** (Nick deferred it until the lead is known); revise until
   approved.
7. **chapters/ch01.md:** scene-by-scene beat sheet matching `chapter-1.md`
   (pre-battle scenes, in-battle moments incl. boss/talk-recruit if any,
   post-battle), each scene with purpose and what changes.
8. **ledger.md:** initial state of each character (what they know, relationships).
9. Critique pass per the skill before committing.

## Acceptance criteria

- [x] Nick approved gate 1 and gate 2 (quotes recorded: gate 1 at the bottom of `docs/story/bible.md`, gate 2 at the bottom of `docs/story/outline.md`).
- [x] All files exist with every required section; no contradiction with `beats.md`.
- [x] Chapter 1 beat sheet fits `chapter-1.md` (roster size, objective).
- [x] `docs/story/README.md` status table updated.

## Completion notes

Worked by Opus 5.5 instead of Fable 5.1, at Nick's request.

**What was done**

- **Gate 1** took four rounds with Nick:
  - **Disgrace:** the Kiryu/Yakuza hook, reworked so the lead comes to regret
    it. The lead took the blame for their best friend Dace, who turns out not
    to be who they thought, and becomes a major antagonist.
  - **Magic:** vows sworn to the memory of dead gods, controlled by a church
    (the Vigil). No singing, and no marks.
  - **Antagonist:** a grieving king who wants his son back.
  - **Mentor:** the old retainer Hollis, the scripted Act 1 loss.
  - **Renaming:** the player can rename the lead. There's a default first
    name, and the family name "Veyne" is fixed.
  - **Dace** is redeemable: he turns in the last act and pays for it.
  - **The cast's humour mix** was approved.
- **Gate 2:**
  - **Chapter 3 is linear**, so the world map opens at Chapter 4.
  - **The twists:** the midpoint is "the prince already came back"
    (Unfinished), and the later twist is "Wren is the bearer". Nick worried A
    was "so creepy", so `outline.md` has binding "sad, not creepy" rules for
    the prince, with C alone as a fallback that needs no Act 1 changes.
  - Nick will give feedback on names and vibes after a longer playtest ("write
    whatever you think works best and isn't totally derivative").
- **Files:**
  - `docs/story/bible.md`
  - `names.md` (new)
  - `outline.md`, with Act 1 as 8 chapters
  - `chapters/ch01.md`
  - `ledger.md`
  - 11 character sheets in `characters/`, named by role id: `lead`,
    `retainer`, `sergeant`, `poacher`, `keeper`, `heretic`, `rival`, `king`,
    `vowmaster`, `red_captain` (the Chapter 1 boss) and `sister`.

**Deviations**

- **The name registry (`docs/story/names.md`) wasn't in the ticket.** Nick
  asked at gate 1 that every name be a variable ("I'm not super sold on these
  names"):
  - Every proper noun has a stable id, and character sheets are named by
    role id, not by name.
  - New ticket **0709** adds a names table in game data and name tokens in
    `.dlg` text, with a validator error for a registered name written
    literally. So a rename in the game is one line. 0702 was already done on
    main when this PR merged main in, so the requirement went into 0709
    instead of editing 0702. 0707 is now blocked by 0709.
  - Ticket **0801** now has the lead's name entry, with default **Rowan**.
  - The `story-writing` skill gained a "Names are variables" section.
- **More gate rounds than planned.** Nick asked to go deeper on the disgrace
  and the twists. All his words are recorded verbatim.
- **Critique pass:** done as a separate reviewer pass. It found about 20
  issues, all fixed:
  - The Veyne seal on Vosse's cart four years ago: Dace stamped the passes
    with Lord Veyne's seal.
  - Harl's death quote gave away Chapter 3.
  - Hollis's confession only existed in a side quest; it's now in Chapter
    5's main story.
  - Dace couldn't be the Door's bearer: he's now gifted, and hid it.
  - Several timeline, ledger and voice mismatches.
  - Crane and Harl both talked in prices; Crane now talks in catalogues.
  - The lead's sheet stated a fixed opinion; it now describes their
    situation only.
- **`_typos.toml`:** added the invented words `Aske`, `Othe` and `mor`, which
  the spell-check read as typos.
- **Design docs updated** (their open sub-questions are now answered):
  - `setting-and-tone.md`: the disgrace, renaming and twist.
  - `magic.md`: what magic is, and where the signature spells are.
  - `world-structure.md`: Chapter 3 is linear; the acts and continents.

**Derivative-risk notes, for Nick's later review**

- Dace follows Yakuza's Nishiki closely. That's intended, since Nick picked
  the Kiryu hook and "turns in the last act and pays for it".
- The Unpaid (veterans turned brigand, with an ex-comrade as the first boss)
  is a common trope; FFT's Death Corps is the closest.
- A dead heir brought back hollow echoes FE Sacred Stones' Vigarde. Ours
  differs: the son is aware, sad and sympathetic, and the father can't refuse
  him.

**Claude's starting rules (gameplay-affecting; Nick may veto)**

- **Talents** (`progression.md`):
  - the lead: Str
  - Hollis: HP
  - Tamsin: Spd
  - Aske: Dex
  - Piers: Res
  - Rue: Mag
  - Dace: Spd
- **Signature spells** (numbers by 0005 or a balance ticket):
  - Piers:
    - *Vigil Light*: a weaker heal at range 2.
    - *Last Rites*: a None-element attack, range 1, few uses, learned later.
  - Rue:
    - *Cinder Oath*: Fire, range 1, high might, few uses.
    - *Kept Promise*: Fire, range 1–3, lower hit, learned later.
  - Crane (enemy): *Closing Word* and *Toll*.
- **The lead ↔ Hollis support starts at C** (a threshold override, since
  they're already close). His A must be reachable before he dies in
  Chapter 8.
- **Harl** is a Brigand boss, and the only Chapter 1 enemy that uses an
  active skill.
- **When extras are introduced** (each chapter ticket confirms with Nick):
  - Chapter 2: Preparations, talk-recruit and villages.
  - Chapter 3: Seize and chests.
  - Chapter 4: the world map, towns, fixed skirmishes and side quests.
  - Chapter 6: elementals (echoes) and battle notes.
- **Act 1 recruits after Chapter 1:**
  - Joss Pellam (Swordsman), Chapter 2 talk-recruit.
  - Gil Parrow (Brawler), Chapter 4.
  - Hedda Ravn (Raider), Chapter 5.
  - Oriel Mast (Sorcerer), Chapter 6.
- **Hollis in Classic mode:** if he dies before Chapter 8, that chapter needs
  an alternate version of his death scene. Flagged for the Chapter 8 script
  ticket.
- **Dace may be playable** briefly in the last Act 3 chapters. Decided by
  those tickets.
- **Default lead first name:** Rowan (a placeholder, like every name).

**Follow-up tickets:** **0709** Names table and name tokens. 0801 was edited (above).
Beat sheets for Chapters 2+ belong to those chapters' future tickets.
