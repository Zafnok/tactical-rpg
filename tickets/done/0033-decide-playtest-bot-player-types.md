---
id: "0033"
title: "Decide: playtest bot player types and what each must achieve"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-30
---

# 0033 — Decide: playtest bot player types and what each must achieve

## Context

Nick's idea (2026-09-30): bots that play a battle the way different kinds of
players would, so we can see whether a battle is tuned right without waiting
for human testers. His first description:

> "the casual is able to clear (assuming they play on casual mode) with
> probably some fallen units (which will come back since they play on casual
> mode). The normal player probably clears with none but takes a while and
> uses many consumables and the hardcore can clear quickly without many
> consumables used"

Already decided by Nick in the same conversation: **bots never use rewind.**
(Technical side, not a question: bots also don't get to peek at the battle's
hidden luck when they plan, the same as a human; see 0504.)

This ticket turns that description into targets a bot run can be checked
against. It unblocks 0506 (the three bots). The bots and the report are built
by 0504–0506; 0507 (a trained bot) and 0508 (comparing bots to Nick's own
play) come after. Run with the `ask-nick` skill. Record the answers in a new
`docs/design/playtest-bots.md` and link it from `docs/design/README.md`.

Word every question as a situation Nick would see in a report, with a worked
example (see how 0006 was asked). The numbers below are options, not
decisions.

## Nick input

**Decision.**

## Questions to ask

### Q1. Which player types?

Example report line: "Chapter 1, Casual bot: won 94 of 100 tries."

**A. Three: Casual, Normal, Hardcore** (Nick's description above).
**B. Three plus a "first-timer"** who makes the mistakes a new player makes
(walks into enemy range, forgets to heal). Like the "new player" testers
studios bring in for a first-time-experience playtest.
**C. Two: "can a casual player win?" and "how fast can an expert win?"**.
Fewer numbers to read.
**Recommendation:** A. A first-timer can be added later.

### Q2. Which mode does each type play?

**A. Casual bot on Casual mode; Normal and Hardcore on Classic** (reads like
Nick's description: "none" fallen only matters in Classic).
**B. Every bot plays both modes** (twice the report, shows how much Casual
mode helps).
**Recommendation:** A.

### Q3. What does "clear" mean for a bot?

A bot plays each try once from the start with fresh luck. It never rewinds.
Humans restart after a game over; bots could too.

**A. First try only.** "Won 94 of 100 tries" means 94 first-try wins.
**B. Up to 3 restarts count as one clear** (closer to how people really
play FE: game over → Retry).
**Recommendation:** A. It's simpler to read, and restarts can be estimated
from the first-try rate.

### Q4. Casual targets

Example: "Casual bot, Casual mode: won 91/100; on average 2 units retreated
(most 5); used 4 items."

**A. Wins at least 9 of 10 tries; any number of retreats** (only the lord
matters, since the lord falling is game over in both modes).
**B. Wins at least 9 of 10; at most 2 retreats on average.**
**C. Wins at least 3 of 4.** A casual player restarts now and then; FE's
Casual mode players do.
**D. Describe your own.**

### Q5. Normal targets

Example: "Normal bot, Classic: won with nobody dying in 82/100; took 14 turns
on average; used 9 items."

**A. Wins with nobody dying in at least 8 of 10 tries. Turns and items are
shown but not judged.**
**B. As A, plus it must be slower and use more items than the Hardcore bot**
(checks the two are really different players).
**C. Describe your own.**

### Q6. Hardcore targets: how is "quickly" judged?

**A. A par turn count per battle**, written in the battle file by whoever
builds the map (like Advance Wars' rank screen, which scores speed against
each map's par). Example: "Hardcore: 8 turns (par 9) ✓".
**B. Relative to the Normal bot**, e.g. at most 60% of its turns and half its
items. No per-map number to maintain.
**C. Shown, not judged.** Nick reads the numbers and decides.
**Recommendation:** A. FE's own tactics ranks (FE6/FE7) also use per-chapter
turn targets.

### Q7. Same targets on every battle?

Every map already has a difficulty tier (Easy/Normal/Hard/Finale) for rewind
charges.

**A. Same targets everywhere.**
**B. Per tier.** Example: on a Finale map the Casual bot may lose more units.
**C. Per battle**, written in each battle file.
**Recommendation:** B, with per-battle overrides allowed.

### Q8. Do bots know about surprise reinforcements?

The bots plan by trying moves on a copy of the battle, so they'd see
reinforcements arrive before a first-time player would.

**A. No: bots play like someone seeing the map for the first time.**
Surprise reinforcements hurt them like they hurt a new player (the famous
FE ambush spawns).
**B. Yes: bots play like someone replaying the map.**
**C. Casual doesn't know; Normal and Hardcore do.**
**Recommendation:** C: hardcore players often know a map already.

### Q9. What happens when a battle misses its targets?

**A. Report only.** Nick reads it.
**B. It blocks the change** (the automatic checks fail until the battle is
retuned).
**C. Report, and block only when even the Hardcore bot can't win** (the map
has become unwinnable).
**Recommendation:** C.

## Acceptance criteria

- [x] Nick answered Q1–Q9 (or chose "describe your own").
- [x] `docs/design/playtest-bots.md` records his words verbatim, then the
      rules in plain language with a worked example report.
- [x] `docs/design/README.md` links it.
- [x] Any change of scope for 0505–0508 that the answers imply is written into
      those tickets.

## Completion notes

Nick answered Q1–Q9 plus two new questions (Q10, Q11) raised by his idea of
autobalancing battles and generating skirmishes to order. Recorded in
`docs/design/playtest-bots.md`, linked from `docs/design/README.md`.

- Q1 A (Casual, Normal, Hardcore), Q2 A, Q3 A (first try only), plus
  per-try statistics of who fell and when, and history across runs.
- Targets: Casual wins ≥60%; Normal wins with nobody dead and ≤3 items ≥50%;
  Hardcore wins with nobody dead and ≤1 item ≥50% ("where veterans would
  end up"). Bands, not floors. Per tier (Q7 B). No bot knows reinforcements
  (Q8 A). Only "unwinnable" blocks (Q9 C).
- Autobalancing: anything for random skirmishes, enemy numbers only for
  fixed skirmishes and side quests. "Lv 35" means a lv 35 battle.

*Claude's starting rules* (Nick may veto):
- Upper band edges are the lower edge +25 points; Easy/Hard/Finale rows of
  the band table are proposed values.
- "Hardcore is faster" is checked as Hardcore's median turns < Normal's; no
  per-map par.
- "Unwinnable" = Casual succeeds under 20% of tries.
- "Each epoch" read as each batch of runs (report history).
- Story battles get the same autobalance limits as fixed skirmishes.

Tickets changed: 0505 (per-try who-fell-when list, history), 0506 (success
rules, bands per tier, Casual ignores deaths, Hardcore stronger than Normal,
no reinforcement knowledge, block only on Casual <20%, workflow now
required), 1008 (pool can be filled by the generator). New: 0509
(autobalance), 0510 (generate a skirmish to order).
