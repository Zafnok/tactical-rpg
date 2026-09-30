//! Battle event sounds (ticket 0424, `docs/design/audio.md` "Sound effects"):
//! which sound cue each battle event plays. The rules are pure functions
//! here; the combat [`Playback`](super::playback::Playback) places strike
//! and cast cues on its timeline, the walk plays a step per tile, and the
//! other events' cues wait in a [`CueQueue`] until the screen's next
//! frame hands them to [`Ctx::audio`](crate::Ctx::audio).

use std::collections::BTreeMap;

use trpg_core::{BattleState, Element, Equipped, Event, Strike, Unit, UnitId, WeaponKind};

use super::mode::WALK_TILES_PER_S;

/// What a fighter strikes with, as far as its sounds go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attack {
    /// A weapon of this kind.
    Weapon(WeaponKind),
    /// An attack spell of this element.
    Spell(Element),
}

/// The sound of `strike`, made with `with` (`None`: unknown):
///
/// - a miss: `miss`;
/// - an Absorb strike (the target heals from its own element): `heal`
///   (Nick, 0424 sign-off);
/// - a hit for 0 damage: `block`;
/// - a weapon hit: `hit_<kind>`, or `crit_physical` on a crit;
/// - a spell hit: `hit_magic`, or on a crit `crit_fire` / `crit_ice`
///   (`hit_magic` for a spell with no element).
///
/// A hit with an unknown attack makes no sound.
pub fn sound_for_strike(strike: &Strike, with: Option<Attack>) -> Option<&'static str> {
    if !strike.hit {
        return Some("miss");
    }
    if strike.healed {
        return Some("heal");
    }
    if strike.damage == 0 {
        return Some("block");
    }
    match (with?, strike.crit) {
        (Attack::Weapon(_), true) => Some("crit_physical"),
        (Attack::Weapon(kind), false) => Some(match kind {
            WeaponKind::Sword => "hit_sword",
            WeaponKind::Spear => "hit_spear",
            WeaponKind::Axe => "hit_axe",
            WeaponKind::Bow => "hit_bow",
            WeaponKind::Gauntlet => "hit_gauntlet",
        }),
        (Attack::Spell(Element::Fire), true) => Some("crit_fire"),
        (Attack::Spell(Element::Ice), true) => Some("crit_ice"),
        (Attack::Spell(_), _) => Some("hit_magic"),
    }
}

/// The sound of a spell of `element` going off: `cast_fire`, `cast_ice`,
/// nothing for a spell with no element.
pub fn cast_sound(element: Element) -> Option<&'static str> {
    match element {
        Element::Fire => Some("cast_fire"),
        Element::Ice => Some("cast_ice"),
        Element::None => None,
    }
}

/// The sound of one step of a unit of movement type `movement_type`
/// (`assets/data/terrain.ron`); fliers make none yet.
pub fn step_sound(movement_type: &str) -> Option<&'static str> {
    match movement_type {
        "foot" => Some("step_foot"),
        "armored" => Some("step_armored"),
        "mounted" => Some("step_mounted"),
        _ => None,
    }
}

/// The step sound of `unit` in `state` (its class's movement type), if
/// it makes one.
pub fn unit_step_sound(state: &BattleState, unit: &Unit) -> Option<&'static str> {
    let class = state.classes().get(&unit.class)?;
    let name = state
        .terrain()
        .movement_types
        .get(usize::from(class.movement_type.0))?;
    step_sound(name)
}

/// What `unit` strikes with when it has `equipped` equipped.
fn attack_of(state: &BattleState, unit: &Unit, equipped: Option<&Equipped>) -> Option<Attack> {
    match equipped? {
        Equipped::Weapon(slot) => {
            let copy = unit.loadout.weapon(*slot)?;
            Some(Attack::Weapon(state.items().weapon(&copy.def)?.kind))
        }
        Equipped::Spell(spell) => Some(Attack::Spell(state.spells().get(spell)?.element)),
    }
}

/// For each [`Event::CombatResolved`] in `events`, in order, what its
/// attacker and defender strike with. `before` is every unit before the
/// command; weapons equipped by the command's [`Event::Equipped`]s count
/// from then on.
pub fn combat_attacks(
    events: &[Event],
    before: &[Unit],
    state: &BattleState,
) -> Vec<[Option<Attack>; 2]> {
    let mut equipped: BTreeMap<UnitId, Option<Equipped>> = before
        .iter()
        .map(|u| (u.id, u.loadout.equipped.clone()))
        .collect();
    let attack = |id: &UnitId, equipped: &BTreeMap<UnitId, Option<Equipped>>| {
        let unit = before.iter().find(|u| u.id == *id)?;
        attack_of(state, unit, equipped.get(id)?.as_ref())
    };
    let mut attacks = Vec::new();
    for event in events {
        match event {
            Event::Equipped { unit, equipped: e } => {
                equipped.insert(*unit, Some(e.clone()));
            }
            Event::CombatResolved {
                attacker, defender, ..
            } => attacks.push([attack(attacker, &equipped), attack(defender, &equipped)]),
            _ => {}
        }
    }
    attacks
}

/// Whether `events` hold a heal of more than 0 HP.
pub fn heals(events: &[Event]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::Healed { amount, .. } if *amount > 0))
}

/// The cues of `events` that no playback or walk plays, each with its
/// delay in seconds: a step per tile of every [`Event::UnitMoved`] (at
/// the walk's pace) except `walked`'s, whose walk was shown; and, unless
/// the command had a combat playback (`combat`), which plays them itself,
/// the cast sound of every [`Event::SpellCast`] and `heal` if a unit was
/// healed.
pub fn event_cues(
    events: &[Event],
    walked: Option<UnitId>,
    combat: bool,
    state: &BattleState,
) -> Vec<(f32, &'static str)> {
    let mut cues = Vec::new();
    for event in events {
        match event {
            Event::UnitMoved { unit, path } if Some(*unit) != walked => {
                let moved = state
                    .unit(*unit)
                    .or_else(|| state.fallen().iter().find(|u| u.id == *unit));
                let Some(cue) = moved.and_then(|u| unit_step_sound(state, u)) else {
                    continue;
                };
                // Tile k (k ≥ 1) is entered k / speed seconds in, as in
                // a walk. Paths are short: the cast is exact.
                #[allow(clippy::cast_precision_loss)]
                cues.extend((1..path.len()).map(|k| (k as f32 / WALK_TILES_PER_S, cue)));
            }
            Event::SpellCast { spell, .. } if !combat => {
                let element = state.spells().get(spell).map(|s| s.element);
                cues.extend(element.and_then(cast_sound).map(|c| (0.0, c)));
            }
            _ => {}
        }
    }
    if !combat && heals(events) {
        cues.push((0.0, "heal"));
    }
    cues
}

/// Cues waiting to be played, each with the seconds it has left.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CueQueue {
    cues: Vec<(f32, &'static str)>,
}

impl CueQueue {
    /// Queues `cues` (delay, cue).
    pub fn extend(&mut self, cues: impl IntoIterator<Item = (f32, &'static str)>) {
        self.cues.extend(cues);
    }

    /// Lets `dt` seconds pass (a bad `dt` counts as 0) and returns the
    /// cues now due, in the order queued.
    pub fn tick(&mut self, dt: f32) -> Vec<&'static str> {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let mut due = Vec::new();
        self.cues.retain_mut(|(left, cue)| {
            *left -= dt;
            let now = *left <= 1e-6;
            if now {
                due.push(*cue);
            }
            !now
        });
        due
    }

    /// Whether nothing waits.
    pub fn is_empty(&self) -> bool {
        self.cues.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::Side;

    use super::*;

    const KINDS: [(WeaponKind, &str); 5] = [
        (WeaponKind::Sword, "hit_sword"),
        (WeaponKind::Spear, "hit_spear"),
        (WeaponKind::Axe, "hit_axe"),
        (WeaponKind::Bow, "hit_bow"),
        (WeaponKind::Gauntlet, "hit_gauntlet"),
    ];

    fn strike(hit: bool, crit: bool, damage: i32, healed: bool) -> Strike {
        Strike {
            by: Side::Attacker,
            hit,
            crit,
            damage,
            healed,
            target_hp_after: 5,
        }
    }

    /// The cues of a miss, a hit, a crit, a 0-damage hit, a 0-damage crit
    /// and an Absorb strike made with `with`.
    fn all(with: Option<Attack>) -> [Option<&'static str>; 6] {
        [
            sound_for_strike(&strike(false, false, 0, false), with),
            sound_for_strike(&strike(true, false, 7, false), with),
            sound_for_strike(&strike(true, true, 21, false), with),
            sound_for_strike(&strike(true, false, 0, false), with),
            sound_for_strike(&strike(true, true, 0, false), with),
            sound_for_strike(&strike(true, false, 7, true), with),
        ]
    }

    #[test]
    fn every_weapon_kind_hits_with_its_own_sound_and_crits_physical() {
        for (kind, hit) in KINDS {
            let with = Some(Attack::Weapon(kind));
            let expected = [
                Some("miss"),
                Some(hit),
                Some("crit_physical"),
                Some("block"),
                Some("block"),
                Some("heal"),
            ];
            assert_eq!(all(with), expected, "{kind:?}");
        }
    }

    #[test]
    fn spells_land_with_hit_magic_and_crit_by_element() {
        let crit = [
            (Element::Fire, "crit_fire"),
            (Element::Ice, "crit_ice"),
            (Element::None, "hit_magic"),
        ];
        for (element, crit) in crit {
            let with = Some(Attack::Spell(element));
            let expected = [
                Some("miss"),
                Some("hit_magic"),
                Some(crit),
                Some("block"),
                Some("block"),
                Some("heal"),
            ];
            assert_eq!(all(with), expected, "{element:?}");
        }
    }

    #[test]
    fn an_unknown_attack_only_misses_blocks_and_absorbs() {
        assert_eq!(
            all(None),
            [
                Some("miss"),
                None,
                None,
                Some("block"),
                Some("block"),
                Some("heal")
            ]
        );
    }

    #[test]
    fn casts_and_steps() {
        assert_eq!(cast_sound(Element::Fire), Some("cast_fire"));
        assert_eq!(cast_sound(Element::Ice), Some("cast_ice"));
        assert_eq!(cast_sound(Element::None), None);
        assert_eq!(step_sound("foot"), Some("step_foot"));
        assert_eq!(step_sound("armored"), Some("step_armored"));
        assert_eq!(step_sound("mounted"), Some("step_mounted"));
        assert_eq!(step_sound("flying"), None);
        assert_eq!(step_sound("swimming"), None);
    }

    #[test]
    fn the_queue_plays_cues_once_their_time_comes() {
        let mut q = CueQueue::default();
        q.extend([(0.0, "a"), (0.1, "b"), (0.2, "c")]);
        assert_eq!(q.tick(0.0), ["a"]);
        assert_eq!(q.tick(f32::NAN), Vec::<&str>::new());
        assert_eq!(q.tick(-1.0), Vec::<&str>::new());
        assert_eq!(q.tick(0.05), Vec::<&str>::new());
        assert_eq!(q.tick(0.05), ["b"]);
        assert!(!q.is_empty());
        assert_eq!(q.tick(1.0), ["c"]);
        assert!(q.is_empty());
    }
}
