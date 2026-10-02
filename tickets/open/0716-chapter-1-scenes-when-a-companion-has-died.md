---
id: "0716"
title: Chapter 1 scenes when a companion has died
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0707", "0715"]
nick_input: sign-off
completed:
---

# 0716 — Chapter 1 scenes when a companion has died

## Context

The Chapter 1 script (0707, `assets/dialogue/ch01.dlg`) is written for a
battle everyone lived through. In Classic a companion who falls is dead
(`docs/design/death-and-difficulty.md`), and today they still talk in the
victory scene: Tamsin closes Harl's eyes, Rue recognises the seal, Piers
says they can't stay, Aske asks for his arrows, Hollis looks at the wax.
0715 gives scripts a way to mark lines by who is still in the army. This
ticket uses it in Chapter 1, following the `story-writing` skill.

The story's rule is already set (0701, approved at the gates): a dead
companion is gone, and scenes are written so "their absence is felt"
(`docs/story/characters/retainer.md`, "Classic permadeath"). Nothing here
is a new design decision.

## Nick input

**Sign-off:** the PR lists, for each companion, what the victory scene does
without them, as text. Nick comments if a beat reads wrong.

## Scope

**In:**
- `ch01_victory`: every line a companion speaks, and every line about what
  a companion is doing, works when that companion is dead. The chapter's
  facts must still reach the player on every path: the order bears the
  Veyne seal; only Dace can use it; they leave for Veyne Hall. Rue's
  suspicion ("I've seen that seal before") is hers alone: if she is dead
  it is not said, and the ledger records that it wasn't.
- `ch01_victory`'s reply choice: a reply answered by a companion needs an
  answer when they are dead (Hollis, Tamsin and Aske answer today).
- In-battle scenes that speak to or about another companion who may have
  fallen first: `ch01_first_turn` (none can have), `ch01_death_keeper` and
  `ch01_retreat_keeper` (Piers speaks to Rue), `ch01_death_heretic` (Rue
  names "the Keeper"), `ch01_boss_engage_sergeant` (only Tamsin; fine).
  Check each and fix the ones that read wrong.
- `ch01_prebattle` and `ch01_intro` need nothing: nobody can be dead yet.
  Say so in the Completion notes rather than leaving it unsaid.
- `docs/story/ledger.md`, "After Chapter 1": for each companion, what is
  different if they died in Chapter 1 (what they never said, who knows
  what instead). Later chapters' scripts read this.
- The critique pass, including one read-through per dead companion and one
  with only the lead and Hollis alive.

**Out (do not do):**
- The engine (0715).
- Chapters 2 and later. Their script tickets handle their own scenes; this
  ticket only records in the ledger what a Chapter 1 death changes.
- A grief scene longer than the budget allows: `ch01_victory` stays at
  about 20 text boxes on every path (`docs/story/chapters/ch01.md`,
  "Budget").
- New reply choices.

## Implementation steps

1. Reread the canon (the skill's canon order), `ch01.dlg` and the sheets of
   all six.
2. For each of the five companions, write down what `ch01_victory` loses
   without them and who or what carries it instead. Starting points:
   - **Tamsin dead:** nobody jokes over Harl. Hollis finds the orders.
     Her "as far as there's pay" is gone; someone notices her horse, or her
     ledger. The ledger went to the lead in her death quote.
   - **Rue dead:** no seal line. Piers's "others will come" loses its
     reason (they came for her too); he can still say the order named
     three and two are standing.
   - **Piers dead:** who explains that the seal goes with the title?
     Hollis won't. Tamsin can ("I thought they took it off you").
   - **Aske dead:** his arrows line goes; the blunt reply needs another
     answer.
   - **Hollis dead:** the hardest one. "Everyone looks at Hollis" becomes
     nobody being left to look at. The spade line goes; someone else
     buries Harl, or nobody does. The line before the reply choice is his.
3. Mark the lines with 0715's block. Keep the all-alive path word for word
   as 0707 wrote it unless the critique finds a reason to change it.
4. Count text boxes on the longest and shortest paths; both within budget.
5. Validate (`cargo test -p trpg-content`), then the critique pass.
6. Update the ledger.

## Acceptance criteria

- [ ] No scene in `ch01.dlg` puts a dead companion on screen or has one
      speak (0715's check passes with its rule on for this file).
- [ ] With any one companion dead, and with all five dead, `ch01_victory`
      still tells the player the three facts in Scope.
- [ ] The all-alive path is unchanged, or each change is listed in the
      Completion notes with its reason.
- [ ] Critique checklist completed and pasted in the Completion notes.
- [ ] Ledger updated.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the all-assets test (the script validates).
- No new code tests: 0715 tests the mechanism.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
