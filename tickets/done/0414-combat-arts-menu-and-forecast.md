---
id: "0414"
title: "Combat Arts in the attack flow: arts list, forecast with arts, durability display"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0312", "0404", "0412"]
nick_input: sign-off
completed: 2026-09-29
---

# 0414 — Combat Arts menu and forecast

## Context

0312 adds Combat Arts to `core` (`docs/design/combat-arts.md`). The player
needs to pick an art when attacking and see what it will do before
committing. 0412 already lets the player cycle combat actives on the
forecast. This ticket turns that into one list of arts **and** combat
actives, as in `combat-arts.md` → *Forecast display*. The UI only shows
`core`'s forecast; it never computes numbers
([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

**Sign-off:** attack a few enemies with arts in Quick Battle. Is it quick to
pick an art, and can you tell what it will do and what it costs?

## Scope

**In:**
- An arts/actives list in the attack flow (0404), after choosing a target.
- The forecast panel with the chosen art applied, a line with its name and
  the durability change, and text notes for non-number effects.
- Durability `left/max` on the weapon line of the forecast and the unit info
  screen (0405).
- Debuff markers (Pinned, Slowed) on the map and info screen, reusing 0412's
  timed-effect markers.
- Enemy- and Other-phase playback shows the name of a boss's or a combat
  green unit's art or active.

**Out (do not do):**
- Core rules (0312).
- Boss AI (0503).
- The blacksmith/repair UI (0409).

## Implementation steps

1. After a target is chosen, show a list under the forecast:
   `Attack` (default), then usable arts, then combat actives. Each row shows
   name, cost (`−4 dur`, `+1 use`) and source (`E`/`D` rank, `weapon`,
   `active`). Arts and actives the unit knows but can't pay for (broken
   weapon, not enough durability) are shown dimmed with the reason. Arts
   that don't fit (e.g. Close Shot only matters with a bow) aren't listed.
   Moving the list cursor updates the forecast live. This replaces 0412's
   `s` cycle for combat actives.
2. The forecast panel adds a first line `‹Art name› (20 → 16)` when an art or
   active is chosen, and shows `core`'s notes as text: `no counter`,
   `pierces`, `pins: Mov −3`, `slows: Spd −3`, `stance: avoid +20`.
   Follow the mockup in `combat-arts.md`, but the numbers part is now the
   forecast Nick chose in 0404 (`look-and-feel.md` → *Attack forecast*: HP
   bars, strikes in order, skull kill mark), which is too wide to sit beside
   the arts list in the side panel; lay the two out together.
3. Weapon lines everywhere (forecast, weapon choice list, info screen) show
   durability `20/20`, and `broken` in the warning colour at 0.
4. Map/info markers for debuffs, with when they expire.
5. Enemy-phase playback: a one-line banner with the art or active name when
   a boss or a green unit uses one (`ArtUsed`/`SkillUsed` events).
6. Help bar (0406) text for the arts list keys.

## Acceptance criteria

- [x] Harness: a Swordsman picks Guard Break against a Brigand; the forecast shows `Guard Break (20 → 16)`, hit 100 and `no counter`, matching `core`'s forecast; after committing, the weapon shows `16/20`.
- [x] Harness: with the weapon at 3/20, Guard Break is dimmed and can't be chosen.
- [x] Snapshots: arts list, forecast with an art, forecast with a note-only art (Pinning Shot), broken weapon line.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness + snapshots as above.

## Completion notes

- **Done.** New `battle::art_list` module: the arts list's lines
  (`Attack`, then the weapon's arts, then the unit's combat actives), costs
  (`−4 dur`, `+1 use`), sources (`E`/`D`, `weapon`, `active`), the dimmed
  reasons, the notes' text and the playback banner. Every line is checked
  with `BattleState::preview_attack`: a line the core accepts can be chosen,
  a line only its cost stops is dimmed with the reason, and anything else
  (a bow art with a sword, a spell active with a weapon, Inspire) isn't
  listed.
  - **Attack flow:** while targeting, the list is a box over the map right
    beside the forecast panel, with the weapon and its durability in its top
    border (`Iron Sword 20/20`). Left/Right (and next/previous unit) pick the
    target, Up/Down move the list (skipping dimmed lines), and the forecast
    follows at once. Confirm attacks with the chosen line. This replaces
    0412's Up/Down ring of combat actives. A unit with nothing but `Attack`
    gets no list, and Up/Down pick targets as before.
  - **Forecast:** `Guard Break (20 → 16)` under the title for an art or an
    active; the weapon's durability under each side's weapon name (`20/20`,
    `(broken)` at 0); the art's non-number notes under the totals in the
    effect colour (`pierces`, `pins: Mov −3`, `slows: Spd −3`,
    `stance: +20 avo`). `no counter` stays in the target's column (0404).
  - **Durability on weapon lines:** the weapon list (`20/20`, `broken` in
    the warning colour) and the info screen (`broken` instead of `0/20`).
  - **Debuffs:** Pinning Shot and Pressure Point use 0412's effect marker on
    the map and its *Effects* block on the info screen (name, `Mov -3`,
    `until … phase`). Nothing new was needed; a test covers a real Pinning
    Shot.
  - **Playback:** when a boss's or a green unit's attack has an
    `ArtUsed`/`SkillUsed` event, the art's or active's name shows in the
    playback box's top border. Enemies don't use arts until 0503 (boss AI),
    and enemy-phase playback is 0502, so for now this is only tested
    directly.
  - **Help bar:** `Left/Right target · Up/Down art · f attack · d back`
    (key names from the keymap).
- **Deviations:**
  - The list isn't a separate step after the target: it's on screen the
    whole time a target is picked, so choosing an art costs no extra press
    (Nick: "is it quick to pick an art?").
  - The list is its own box beside the panel instead of *under* the
    forecast inside it: at 30 columns the panel can't fit a name, `−4 dur`
    and `active` on one line.
  - The forecast keeps `(broken)` as Nick chose in 0404 (`look-and-feel.md`);
    the lists show `broken`.
  - The stance note reads `stance: +20 avo`, the wording the info screen
    already uses for skills, not `stance: avoid +20`.
- **Follow-up:** 0426 now also covers Close Shot. Targets still come from the
  plain attack, so Close Shot's adjacent targets (and Long Shot's far ones)
  are never offered. Close Shot is listed at distance 2, where it only costs
  durability and hit.
- **Existing tests/snapshots updated on purpose:** the lord and the archer
  know arts, so their forecasts now show the list and the help line reads
  `Up/Down art`; weapon durability under the forecast's weapon names; the
  info screen shows `broken` for a 0/20 weapon. 0412's ring tests were
  replaced by list tests (`art_tests.rs`).
- **Claude's starting rules (all presentation; please agree or veto):**
  - The list sits on the half of the map away from the attacker so it
    doesn't cover the fight.
  - When you switch targets, the chosen art stays if it can still be used
    on the new target; otherwise it goes back to `Attack`.
  - Dimmed reasons read `broken`, `3 dur left` (or `1 uses left` for
    spells).
  - Only a boss's or a green unit's art gets the playback banner, since you
    already know which art your own units use.
