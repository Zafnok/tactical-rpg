---
id: "0001"
title: "Decide: unit stats and combat math"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
---

# 0001 — Decide: unit stats and combat math

## Context

Everything in combat (ticket 0304), the unit model (0302), the info panels
(0405) and level ups (0601) depends on which stats exist and how they
interact. This is the most important decision to make early.

Run this with the `ask-nick` skill. Present the options below (you may refine
wording), get Nick's answer, and record it.

## Nick input

**Decision.** Nick picks an option (or describes his own) for each question.

## Questions to ask

### Q1. Which stats should units have?

**A. Classic Fire Emblem (GBA / Path of Radiance)**
HP, Strength, Magic, Skill, Speed, Luck, Defense, Resistance, Move (+ Constitution for weapon weight/rescue).
In FE: damage = Str + weapon might − enemy Def. Skill drives hit and crit, Speed
drives dodge and "doubling" (attacking twice if 4+ faster), Luck shaves enemy
crits. Feel: every stat on the sheet matters; easy to read.

**B. Streamlined FE (Engage-like)**
HP, Str, Mag, Dex, Spd, Def, Res, Move. Luck removed, Build/Con folded in.
Feel: same FE feel, fewer numbers — nice for a compact ASCII side panel.

**C. Final Fantasy Tactics**
HP, MP, Physical Attack, Magic Attack, Speed, Move, Jump, Brave, Faith. Brave
affects reaction abilities and bare-hand damage, Faith scales magic dealt *and
received*. Feel: deep build-crafting, more opaque, grindier.

**D. Minimal (Advance Wars / Into the Breach)**
HP, Attack, Move (+ maybe Defense). No misses; damage is predictable.
Feel: chess-like, zero RNG frustration, less "character sheet" attachment.

**Recommendation:** A or B — Nick asked for Fire Emblem, and level-up stat
growth is where FE's attachment to units comes from.

### Q2. How random should combat be?

**A. FE "true hit" (GBA onwards)** — hit rolls use the average of two random
numbers, so a displayed 80% actually hits ~92% of the time and 20% hits ~8%.
Feel: high numbers feel reliable, low numbers feel like long shots.
**B. Straight dice (FE1–5, XCOM)** — displayed % is exact. Feel: honest but
"95% and missed" moments happen.
**C. No randomness in hits (Into the Breach / Advance Wars)** — attacks always
hit; only damage values matter. Feel: pure planning.
**D. Randomness only in crits/level-ups** — hits always land, crits still roll.

**Recommendation:** A.

### Q3. Doubling / follow-up attacks

**A. FE doubling** — if your Speed ≥ enemy Speed + 4 (tunable), you strike twice.
**B. None** — one attack each; Speed only affects dodge.
**C. FFT-style** — Speed determines how often you get turns instead.

**Recommendation:** A (depends on Q1/turn structure in 0002).

## What to record

Create `docs/design/stats-and-combat.md` containing:

- Nick's words verbatim.
- The final **stat list**, each with a one-line meaning and valid range
  (e.g. `Str: 0..=30 base, cap per class`).
- **Combat formulas** written as exact arithmetic, using integer maths, e.g.
  `damage = max(0, atk - def)`, `hit = clamp(weapon_hit + skl*2 + lck/2 - avoid, 0, 100)`,
  including rounding rules, the doubling threshold, crit multiplier, and the
  RNG model from Q2 with the exact roll procedure.
- Terrain effects on combat (def/avoid bonuses) — propose FE defaults
  (forest +1 Def +20 Avoid, fort +2/+20, mountain +2/+30…) marked *tunable*.
- **Three worked examples** (attacker stats + defender stats → forecast numbers)
  that ticket 0304 will turn into table-driven tests.
- Open sub-questions deferred (e.g. skills, weapon weight) if any.

Update `docs/design/README.md`. If the answer changes the steps of tickets
0302, 0304, 0405, 0601, edit them in this PR.

## Acceptance criteria

- [x] Nick answered Q1–Q3 (or described his own system).
- [x] `docs/design/stats-and-combat.md` exists with stat list, exact formulas,
      RNG procedure, terrain defaults, three worked examples.
- [x] `docs/design/README.md` table updated.
- [x] Downstream tickets adjusted if needed.
- [x] Ticket archived to `tickets/done/`.

## Completion notes

Nick picked **Streamlined FE** stats (HP, Str, Mag, Dex, Spd, Def, Res, Mov),
**FE true hit (2RN)**, and his own take on doubling: up to **4 strikes**, with
3x/4x very rare. Recorded in `docs/design/stats-and-combat.md` with exact
formulas, the roll procedure, *tunable* terrain defaults and three worked
examples (one with a scripted-roll resolution trace).

- Strike thresholds `diff ≥ 4 / 14 / 24` → 2 / 3 / 4 strikes are **placeholders**:
  Nick deferred the number scale (small vs FE-sized vs huge) to after the
  playtest. New ticket **0013** decides it; 0302 now uses one `StatValue` alias.
- Luck's crit-avoid role goes to Dex (`crit − defender.Dex / 4`, *tunable*).
- `as_bonus` hook left at 0; how gear / weapon skill feed it is now a
  follow-up question added to ticket 0003.
- Downstream edits: 0003 (attack-speed sub-question), 0304 (strike order up
  to 4, 2RN procedure, boundary tests), 0302 (`StatValue` alias). 0405, 0601
  needed no change.
