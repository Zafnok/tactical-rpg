# The lead: [first name] Veyne

Id: `lead` (portraits `lead_m` / `lead_f`) · Default first name: **Rowan**
(both genders) · [Name registry](../names.md)

This is a **Persona-style lead** (`docs/design/setting-and-tone.md`, "The
lead"). The player chooses the gender and may rename the lead (Nick, gate 1:
"A"). The family name **Veyne** is fixed, and most characters simply call the
lead "Veyne". Background and situation are fixed; **personality and voice are
not**. The player supplies them through reply tones.

## Role and class

- **Role:** the lord. Their fall is game over.
- **Class:** **Exile** (tier 1 of the lord-only line, `progression.md`), a
  sword foot soldier who is good at everything. The name fits: the lead *is*
  an exile.
- **Talent:** Str (*Claude's pick*, so the lead can carry fights despite the
  Exile's slower build).
- **No spells:** the lead isn't gifted, and the lord has no personal spell
  (`progression.md`, "nothing else on top").

## Background

- **Age 21.** Heir of House Veyne, an old family of the Ardevali heartland.
  The lead grew up at Veyne Hall with **Dace Marr**, son of the house's guard
  captain **Hollis Marr**. They were raised side by side, closer than
  brothers or sisters, though everyone knew one was the heir and one wasn't.
- **Four years ago**, at 17, the lead was at the mill on the night Dace killed
  Harrick Vosse, the king's man, to get his sister Wren out of Vosse's cart.
  Dace begged: "They'll hang me. They won't hang you." The lead confessed to
  the killing.
- Lord Veyne gave up the title and lands to buy his heir's life. The lead was
  exiled to the Thornmarch, forbidden to leave it, bear arms in the king's
  lands, or use the Veyne name or crest. Lord Veyne died the next winter.
  Hollis followed the lead into exile. Nobody ordered him to.
- **What the lead has been told** (and has had no reason to doubt): Dace
  killed Vosse to save his sister, and that's all there was to it. How the lead
  *feels* about the confession is the player's to decide.
- **What the lead doesn't know:** Dace had been pointing out other families'
  gifted children to Vosse for a year before that night, under Vosse's threat
  and for Vosse's silver, stamping the cart's passes with the Veyne seal.

## Situation at the start

- Lives in Harrowby, a Thornmarch hamlet, in a rented house with Hollis:
  chopping wood, fixing the mill, training with Hollis in the yard with
  wooden swords (steel is forbidden, and they keep two real ones under
  the floor).
- Dace, meanwhile, is a war hero, captain of the king's Hounds, and holds the
  Veyne lands and title. He hasn't written in four years.
- Wren, Hollis's daughter, was tested by the local keeper (Piers) and taken
  by the Vigil three years ago. The family was told she died at the
  Ashfields. The lead and Hollis were there when she was taken, and couldn't
  stop it.
- The hamlet likes the lead the way you like a stray lord: with jokes,
  borrowed tools and suspicion.

## Want and pressure

- **Want (Act 1):** to find out why Dace sent killers with the Veyne seal,
  from Dace's own mouth.
- **Pressure:**
  - Every step out of the Thornmarch breaks the terms of exile, so the lead
    becomes an outlaw from Chapter 1 on.
  - The lead's confession is the foundation of Dace's whole life. Tearing it
    down means admitting the sacrifice fed a monster.
- **Across the game:** the story asks the lead one question in many forms.
  Was saving Dace right, knowing what he was? What do you owe someone you
  love? The player answers through tones, and the plot doesn't branch.

## Arc (situation, not personality)

| Point | Situation |
| ----- | --------- |
| Start | Exiled heir, living under a false quiet, loyal to a lie. |
| Act 1 | Breaks exile; learns the truth about Dace; loses Hollis; flees overseas as a traitor to the crown. |
| Act 2 | A stranger in the Jade Reach, leading people who chose them rather than a house; learns how the Door can be shut. |
| Act 3 | Returns to end the war and close the Door; faces Dace as he turns, and the king as he lets go. |
| End | Whether the lead takes back the Veyne name, refuses it or makes it mean something new is the player's tone, never a branch. |

## Reply tones

Two or three of these per choice point, at most about 1–3 choice points per
chapter (`setting-and-tone.md`). Tones are *registers*, not personalities.
Each option is at most 60 characters (0708).

| Tone | What it sounds like | How the cast tends to react |
| ---- | ------------------- | --------------------------- |
| **Earnest** | Says what they feel, plainly and kindly. | Piers and Aske soften; Tamsin teases; Hollis is gruffly proud. |
| **Wry** | Deflects with dry humour; noble irony. | Tamsin and Rue light up; Hollis sighs; Aske doesn't get it. |
| **Blunt** | Short, hard and practical. | Aske and Hollis approve; Piers winces; Rue pushes back. |

**Sample choice (Chapter 1, after reading Dace's letter):**

```
@choice
* earnest: He's warning us. He still cares, Hollis.
  retainer[sad]: Aye. That's what worries me.
* wry: Four years of silence, and now he's urgent.
  retainer[happy]: Ha. He always did write like a man owed money.
* blunt: What does he want?
  retainer[angry]: Us gone. The question is why.
@endchoice
```

(Syntax per ticket 0708; the text is illustrative. The real lines are written
in 0707.)

## Relationships

| With | Now | Why |
| ---- | --- | --- |
| Dace (`rival`) | Four years of silence | Raised together; the last thing the lead did for Dace was give up everything |
| Hollis (`retainer`) | Family in all but name | He followed the lead into exile and raised them after Lord Veyne died |
| Tamsin (`sergeant`) | Easy, joking friendship | She guards Harrowby for bread and ale and calls the lead "Your Former Grace" |
| Aske (`poacher`) | Wary | He hates Ardevali nobles, and the lead is one, title or no |
| Piers (`keeper`) | Warm, with one splinter | He tested Wren and sent her name to the Vigil |
| Rue (`heretic`) | Hostile at first | She saw Dace at the mill and thinks the lead was in on it |

## Voice notes

The lead speaks rarely, in short neutral lines (at most 40 characters outside
choices, 0708): "Let's move." "Hollis. Now." "I'm listening." No opinions,
jokes or cruelty the player didn't pick. Other characters carry scenes, and
the lead's presence is shown by how others speak *to* them.

## Portrait brief (`lead_m` and `lead_f`: same age, costume and colours)

- **Silhouette:** shoulder-length hair tied back loosely (both versions), a
  high-collared, travel-worn noble coat, and a sword grip over the left
  shoulder. It reads as "a noble who's been living rough".
- **Hair:** dark chestnut. `lead_m`: short beard shadow, a few loose strands.
  `lead_f`: the same tie, with a longer fringe on the left.
- **Face:** 21, lean, tired eyes, one small pale scar through the right
  eyebrow (from Hollis's wooden sword, a running joke). Light shading, since
  the lead is young.
- **Clothing:** a faded Veyne-blue coat with the house crest **cut off** the
  chest, leaving a clean patch of brighter blue where it was. Underneath, a
  plain undyed shirt and a leather baldric.
- **Colours:** faded royal blue (coat), brighter blue (crest patch), undyed
  linen, brown leather, and a steel-grey sword grip.
- **Expressions:** `neutral`, `happy`, `angry`, `sad`, `surprised`, plus
  `determined` (jaw set, brows level, for choice-point close-ups).

## Support partners

In lead supports the lead follows the Persona rules: few lines, at most one
reply choice per conversation (`supports.md`).

| Partner | Kind | What it's about |
| ------- | ---- | --------------- |
| Hollis (`retainer`) | Family / mentorship | Four years of exile in two rooms. What Hollis never says about Dace. Starts at C, because they're already close (a threshold override). His A must be reachable before the end of Act 1. |
| Tamsin (`sergeant`) | Friendship | She keeps a ledger of what the crown owes her; the lead's name is in it. What is loyalty worth if nobody pays for it? |
| Aske (`poacher`) | Rivalry → friendship | A Brennish boy and an Ardevali noble. He wants to hate the lead properly and keeps failing. |
| Piers (`keeper`) | Friendship | He tested Wren. The lead has never said whether they forgive him. |
| Rue (`heretic`) | Rivalry → trust | She was sure the lead was Dace's accomplice. Earning her trust is slow on purpose. |
