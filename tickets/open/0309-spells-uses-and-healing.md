---
id: "0309"
title: "Spells: spell data, spell lists, uses per battle, Cast action, healing"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0004", "0302", "0304", "0306"]
nick_input: none
completed:
---

# 0309 — Spells, uses per battle, Cast, healing

## Context

Nick's magic design (`docs/design/magic.md`, ticket 0004): magic is **innate
spells**, not loadout items. Each spell has **uses per battle** that refill
every battle. Units learn spells from their class list (and keep them after
a class change) plus 1–2 personal spells. Tier-3+ magic classes have 0 weapon
slots. White magic heals and gives EXP. Combat maths for attack spells and
elemental affinities are already in 0304. Loadout and equipping are in 0306.
This ticket adds spells on top, through the command/event system
([ADR-0004](../../docs/adr/0004-crate-architecture.md)). Terrain-changing
tile casts are 0310.

## Nick input

None (all rules are in `magic.md`).

## Scope

**In:**
- `assets/data/spells.ron` and `core::spell`.
- Spell lists on units: class spells learned by level, personal spells.
- `uses_left` per spell, refilled at `BattleState::new`.
- Equipping an attack spell (it becomes the counter attack).
- `UnitAction::Cast { spell, target: CastTarget::Unit(UnitId) }` for attack
  and heal spells.
- Building 0304's `CombatantInput` from a spell.
- Id validation of the class and character spell lists from 0302.

**Out (do not do):**
- Tile casts and terrain changes (0310).
- UI (0410).
- EXP amounts (0601 implements `exp_for_heal`; just emit the events it needs).
- AI use of spells (0501).
- Push spells, and more elements.

## Implementation steps

1. `spells.ron`: the starter spells table from `magic.md` with exact numbers
   (Fire, Frost, Force, Heal, Mend). `core::spell`:
   `SpellId`, `SpellDef { id, name, kind: SpellKind { Attack { might, hit, crit, effective }, Heal { heal_power } }, element: Element, min_range, max_range, uses, terrain_effect: Option<TerrainEffectId> }`.
   Load and validate it in `content`: ids unique, ranges `min ≤ max`,
   `uses ≥ 1`, and every class and character spell id exists.
2. `Unit::known_spells(class_table)`: the union of class spells whose level
   ≤ the unit's **class level** in that class (from `class_records`, for any
   class it has unlocked; `progression.md`) and personal spells whose level
   is ≤ the unit's **character level**. Store learned spells
   on the unit as `learned: BTreeSet<SpellId>`, so a class change keeps them.
   Add a `learn_new_spells()` helper that 0601/0603 call after a level up,
   class level up, promotion or reclass. Test it directly.
3. `SpellState { uses_left: BTreeMap<SpellId, u8> }` on the battle unit,
   filled to each spell's `uses` in `BattleState::new` (and by 0801's
   `battle_setup`, if that exists by now).
4. The equipped attack becomes `Equipped::{ Weapon(slot), Spell(SpellId) }`,
   extending 0306's `Loadout.equipped`. `Command::Equip` accepts a known
   attack spell. Attacking or casting with an attack spell equips it.
5. `UnitAction::Cast { spell, target: CastTarget::Unit(id) }`:
   - **Attack spell:** validate that the spell is known, `uses_left ≥ 1`, and
     the target is hostile, alive and in the spell's range from `dest`. Then
     run 0304's forecast/resolve with the spell's `WeaponStats` (weight 0,
     rank bonus 0, element set). Spend **1 use per combat**, no matter how
     many strikes. A defender that counters with an equipped spell spends 1
     of its uses too, and can't counter if it has 0 uses left. Events:
     `SpellCast`, `CombatResolved` (as for weapons), `SpellUsesChanged`.
   - **Heal spell:** the target must be an ally of the caster (not the caster
     itself), in range, with HP below max. `amount = min(heal_power + Mag, max − hp)`,
     no roll, 1 use, ends the action. Events: `SpellCast`,
     `Healed { target, amount }` (the same event as 0306), `SpellUsesChanged`.
6. Weapon slots: 0306's loadout validation already honours
   `class.weapon_slots`. Add promotion handling: when the new class has fewer
   slots, the extra weapons go to the stock (the helper is called by 0603).
7. Worked examples M3 and M4 from `magic.md`, plus M1 applied through
   `BattleState::apply`, go in tests.

## Acceptance criteria

- [ ] `spells.ron` matches `magic.md` (a test compares at least 3 spells).
- [ ] Uses refill at battle start; one use is spent per combat (a doubling mage spends 1); a spell at 0 uses can't be cast or counter (tests).
- [ ] Heal reproduces M3 (the amount is capped by missing HP; a full-HP ally and self are invalid targets, and the state is unchanged).
- [ ] M4: an equipped spell at 0 uses gives no counter; after `Equip` to Force, the unit counters.
- [ ] Learned spells survive a class change; a 0-slot class can't hold weapons.
- [ ] Every new `CommandError` leaves the state unchanged (test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule above, the M1/M3/M4 examples, the validators.
- Property: `uses_left` never exceeds `uses` and never underflows; HP stays
  within `0..=max` after any heal. Extend 0305's random-command property test
  to include `Cast` and spell `Equip`.

## Completion notes
