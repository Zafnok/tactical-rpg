---
id: "0312"
title: "Combat Arts: art data, learning by rank, costs and effects in forecast and resolution"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0014", "0304", "0305", "0306", "0311"]
nick_input: none
completed:
---

# 0312 — Combat Arts rules

## Context

Ticket 0014 decided Combat Arts (`docs/design/combat-arts.md`): weapon
techniques learned by **weapon rank** (plus special weapons' own arts), paid
once per combat with the attacking weapon's **durability**, boosting **every
strike** of that combat. Some leave a **stance** (on the user, until the start
of its next phase) or a **debuff** (on the target, until the end of the
target's next phase). Chapter 1 has 10 arts, two per weapon kind at ranks E
and D. This ticket adds them to `core` through `Command` → `Event`s
([ADR-0004](../../docs/adr/0004-crate-architecture.md)), hooking into 0304's
forecast/resolve, 0306's durability and weapon EXP, and 0311's cost-paying
helper and timed effects.

## Nick input

None. Every rule and number is in `combat-arts.md`.

## Scope

**In:**
- `assets/data/arts.ron` with the 10 Chapter 1 arts; `content` validation.
- `core::art`: `ArtDef`, `ArtId`, `ArtEffect`.
- Rank arts per weapon kind (known when rank ≥ art rank), and weapon arts
  (`arts: [ArtId]` on weapon data, empty for every Chapter 1 weapon).
- `UnitAction::Attack { …, art: Option<ArtId> }`, with "one art or one combat
  active per attack".
- Every effect in the table: Flowing Cut, Guard Break, Unhorse, Line Pierce,
  Crushing Swing, Armor Cleave, Close Shot, Pinning Shot, Pressure Point,
  Sidestep.
- Debuff timed effects ("until the end of the target's next phase") beside
  0311's stance effects.
- Doubled weapon-EXP base for art combats (the damage bonus isn't doubled).
- The `boss` flag check: only boss enemies may issue an attack with an art or
  active (state unchanged otherwise). The AI choosing them is 0503.

**Out (do not do):**
- UI (0414).
- Boss AI (0503).
- Arts for ranks C–S and special weapons' arts (0018 decides them first).
- Changes to class actives beyond what 0311 did.

## Implementation steps

1. `core::art`:
   - `ArtDef { id, name, kind: WeaponKind, rank: Option<WeaponRank>, cost: u32, effect: ArtEffect }`.
     `rank: None` means a weapon art (only via a weapon's `arts` list).
   - `ArtEffect` fields, all optional/defaulted: `hit_bonus: i32`,
     `sword_followup: Option<(num, den)>` (Flowing Cut `(3, 2)`),
     `no_counter: bool`, `extra_effective: Vec<(Tag, mult)>`,
     `effective_override: Option<(Tag, mult)>` (Unhorse `(Mounted, 3)`),
     `axe_min_damage: Option<u32>`, `min_range_override: Option<u32>`,
     `on_first_hit: Option<Debuff>` (`Mov −3`, `Spd −3`),
     `stance: Option<StatMods>` (Sidestep avoid +20), `pierce: bool`.
     A small struct keeps the data flat in RON; don't build a scripting system.
2. `arts.ron` + validation in `content`: unique ids; every art's kind is a
   real weapon kind; `cost ≥ 1`; every weapon's `arts` ids exist and match
   its kind.
3. `Unit::usable_arts(weapon, class_table) -> Vec<ArtId>`: rank arts of the
   weapon's kind with `rank ≤ unit rank` (only if the current class can
   wield that kind), plus the weapon's own arts. Filter out all of them if the
   weapon is broken or `durability_left < cost`.
4. Paying: reuse 0311's `core::skill::pay_cost` (durability, paid once when
   committed, even if every strike misses; a weapon brought to 0 breaks
   **after** the combat, which is still fought as unbroken).
5. Forecast (0304): add an `art: Option<&ArtDef>` input for the attacker.
   Apply the hit bonus, sword follow-up ratio, axe minimum, effectiveness
   changes (largest multiplier still wins) and min-range override to **all**
   the attacker's strikes. `no_counter` → the defender side is `None`. Add
   `art: Option<ArtId>`, `durability: (before, after)` and a list of
   non-number notes (`NoCounter`, `Pierces`, `Pins(3)`, `Slows(3)`,
   `Stance`) to the forecast output for the UI. Keep the formulas in one
   place; the art only changes their inputs.
6. Resolution: debuff on the **first** hit only; stance applied when the
   attack is committed. Line Pierce: after the main combat, if the attacker
   is alive and the tile `target + (target − attacker)` holds a unit hostile
   to the attacker, resolve one strike (2RN, then crit roll) against it with
   no counter and no follow-ups. It still happens if the main target fell.
   It gives its own unit-EXP/CP award as a second combat.
7. Timed effects: add a `Debuff` expiry kind, "end of the target's side's
   next phase", to 0311's effect list. Mov/Spd debuffs never take the stat
   below 0. The same debuff refreshes and doesn't stack. Movement ranges, the
   danger zone and attack speed read the modified stats.
8. Weapon EXP (0306's formula): an art combat passes `used_art = true`, so
   the base doubles (4 / 2); `dealt / 5` is added unchanged and includes a
   Line Pierce strike's damage.
9. Events: `ArtUsed { unit, art, weapon, durability_before, durability_after }`,
   reusing `EffectApplied`/`EffectExpired`, `ItemBroke` and the combat events.

## Acceptance criteria

- [ ] `arts.ron` matches `combat-arts.md`'s table (test checks all 10: kind, rank, cost).
- [ ] `combat-arts.md` worked example: W1 with Flowing Cut gives follow-up 13; with Guard Break hit 100 and no counter (table-driven test).
- [ ] One unit test per art proving its effect on the forecast or the state (Unhorse ×3 vs Mounted and plain vs others; Armor Cleave vs Armored; Close Shot at distance 1; Line Pierce hits the unit behind, no pierce on a diagonal, still pierces after a kill; Pinning Shot/Pressure Point expire at the end of the target's next phase; Sidestep lasts through the enemy phase).
- [ ] An art can't be used with a broken weapon, with `durability_left < cost`, on a counter, or together with a combat active (state unchanged, tests).
- [ ] A weapon brought to exactly 0 by an art fights that combat unbroken and then emits `ItemBroke` (test).
- [ ] A non-boss enemy's attack command with an art is rejected (test).
- [ ] Weapon EXP for art combats: base doubled, damage bonus not (test with the `combat-arts.md` worked example: 5 / 8 / 7).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each art; learning by rank and after reclass (known but unusable without the kind); costs; break-after; debuff/stance expiry; weapon EXP.
- Property: extend 0305's random-command test with art attacks. Durability never underflows, HP stays in `0..=max`, Mov/Spd never go below 0, and the forecast with an art always matches resolution's strike count.

## Completion notes
