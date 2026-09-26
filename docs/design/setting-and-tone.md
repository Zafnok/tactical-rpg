# Setting, tone and the lead

Decided: 2026-09-25
Source: ticket 0007

Nick's full answers are quoted verbatim in
[`docs/story/beats.md`](../story/beats.md) (canon). This file is the rule set
derived from them, which the story pipeline (0701+) and the dialogue/portrait
tickets implement.

## Nick's words (summary quotes)

> **Setting:** "1A to start, with potential for additional continents that might
> feel different (i.e. start in Europe-like countries / army compositions,
> travel to Asia and encounter wuxia inspired encounters... etc)"
>
> **Tone:** "2B"
>
> **Who we follow:** "3A to start. It would be nice to aim for C eventually but
> let's get one lord down solidly before expanding scope. But this doesn't mean
> the supporting cast should be cast aside, they should also have some arcs or
> role to play in the story etc as well as support conversations"
>
> **The lord:** "L1 C" (disgraced/exiled noble) · "I'm thinking 18-25" · final
> shape "B" (Persona-style lead, player chooses gender).
>
> **Ending:** "should not be a tragedy. It should feel resolved and like a
> hardfought victory."
>
> **Off-limits:** "No romance whatsoever, or if there is any, it should be
> pre-established not built up over time. Avoid shipping."

## Q1. Setting — Fire Emblem-style medieval fantasy war

- The game **opens** in a classic medieval fantasy war: European-style kingdoms
  with European-style armies (knights, cavalry, archers, mages, clergy).
- The world has **other continents that feel different**. Nick's example:
  travel to an Asian-inspired continent with wuxia-inspired encounters. These
  are *potential*: they're not needed for Act 1, but the bible (0701) must leave
  room for them (the world is bigger than the starting kingdoms, and the
  starting kingdoms know that other lands exist). How and when the player
  travels there: decided in 0008 ([`world-structure.md`](world-structure.md)):
  each continent is a new act with its own world map.
- The kingdoms' politics may be messy. Tone B allows it, and a disgraced lead
  implies a court that can wrong people.

## Q2. Tone — dark with warmth and humour

Like FE Path of Radiance and Triangle Strategy: real losses, hard choices and a
corrupt or broken system, with character warmth, banter and jokes between the
hard beats.

- **The ending is not a tragedy.** It is resolved and feels like a hard-fought
  victory. Nick fixed this.
- **Unavoidable story losses are allowed**, including someone close to the lead
  (Nick's reference: Expedition 33 at the end of Act 1). These are scripted and
  separate from Classic-mode permadeath (`death-and-difficulty.md`).
- **No romance.** There are no romance flags, no romance built up over time and
  no shipping, and no S-rank/marriage supports. The only exception is a
  relationship that already exists before the story starts (e.g. a married
  NPC couple), and that is never developed into a romance arc on screen.

## Q3. Who the player follows — a single lord

- **One lord.** Their fall is game over (`death-and-difficulty.md`).
- An ensemble with rotating viewpoints is a *possible later goal*
  ("aim for C eventually"). It is out of scope until Nick asks for it.
- **The supporting cast is not scenery.** Each main companion has their own arc
  or story role that the main plot advances, plus support conversations (the
  system is decided in 0010).

## The lead (the lord)

| Aspect | Decision | Fixed by |
| ------ | -------- | -------- |
| Background | **Disgraced or exiled noble.** What happened, and whether it was deserved, is **not yet decided**: 0701 proposes options to Nick at gate 1 | Nick |
| Age | **18–25** | Nick |
| Gender | **Player chooses** (male/female) at New Game | Nick |
| Personality | **Persona-style lead.** Defined background, look and situation; few spoken lines; the player shapes the personality through reply choices | Nick |
| Name | Player can rename? **Open**, asked at 0701 gate 1 | — |

### Rules for writing the lead

1. **Few lines.** The lead speaks mostly through reply choices, plus short,
   neutral lines when the scene needs them ("Let's move."). Other characters
   carry scenes, like the Persona casts do.
2. **Reply choices at key moments.** Typically 2–3 options with distinct tones
   (e.g. earnest / wry / blunt). The other characters react differently in a
   line or two, then the scene continues the same way. **Choices never branch
   the plot**, recruit or lose units, or change the ending. Budget: at most
   about 1–3 choice points per chapter (*tunable*). Players should feel they
   have a voice without every scene becoming a menu.
3. **The lead's personality is never fixed by the script.** No cruelty, jokes
   or strong opinions that the player didn't pick. Their *situation* (exiled,
   wronged, carrying a title they lost) is defined, and gives the story weight.
4. **Gender-neutral script.** Lines refer to the lead by name or with pronoun
   tokens, and the game fills in the chosen gender. No line or scene depends
   on the lead's gender. The lead has two portrait sets (one per gender) with
   the same silhouette language, age and costume.
5. Because the lead is quiet, **the supporting cast must be vivid.** Their arcs
   and voices are where the writing effort goes.

## Vibe references

Nick's list, as inspiration and **not** as beats to copy: Expedition 33,
Persona 5 Royal, Persona 3 Reload, Cyberpunk 2077, Disco Elysium, NieR:Automata,
the Yakuza series, The Witcher 3, Resident Evil, Dark Souls lore, Baldur's
Gate 3's choices.

Common patterns Claude reads in that list (an interpretation for 0701 to use,
not canon):

- **A personal stake inside a bigger, corrupt or dying system.** The hero has a
  wound or duty and the world around them is rotten (Persona, Cyberpunk,
  Witcher, Yakuza, Disco Elysium).
- **Found family makes the fight worth it.** The bonds of the party or crew are
  the emotional core (Expedition 33, Persona, Yakuza, BG3).
- **Loss that costs something real, followed by a hard-won ending that isn't
  hopeless** (Expedition 33, NieR, P3R).
- **Lore that rewards curiosity.** The history is told in fragments (item
  descriptions, ruins, NPCs) instead of up-front exposition (Dark Souls,
  NieR).
- **Choices that let the player express who they are** (BG3, Disco Elysium,
  Persona dialogue).
- **Absurd humour alongside sincere drama** (Yakuza, Persona, Disco Elysium).

## Open sub-questions (deferred)

- **The lead's disgrace and the inciting incident.** 0701 gate 1 proposes 2–3
  options (Nick: "we can expand more on what exact inciting incident happened").
- **The midpoint twist.** Nick said "Same as 1": it depends on the lead, so
  0701 proposes options at gate 2 (outline).
- **Characters Nick imagines.** Nick asked for "more leading questions": 0701
  gate 1 asks him leading questions about the cast (e.g. the mentor figure,
  rival, antagonist's motive) before the sheets are finalised.
- **Can the player rename the lead?** Asked at 0701 gate 1.
- **Other continents:** how travel happens is decided (0008: a new act and a
  new world map per continent); how many and when is for the outline (0701).
