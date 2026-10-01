//! The Combat Arts list of the attack flow (ticket 0414,
//! `docs/design/combat-arts.md` → *Forecast display*): while a target is
//! chosen, `Attack`, then the arts the attacking weapon can use, then the
//! unit's combat actives, each with its cost and source. What can't be paid
//! for is dimmed with the reason; what doesn't fit the attack isn't listed.
//! Every check is [`BattleState::preview_attack`], so the list never offers
//! what the core would refuse (ADR-0004). An attack spell cast at a unit
//! (0410) has the same list: `Attack`, then the unit's spell actives.

use trpg_core::{
    ArtDef, ArtId, ArtNote, BattleState, CastTarget, CommandError, CostError, Equipped, Event,
    Faction, Pos, SkillCost, SkillId, StatKind, Unit, UnitAction, UnitId,
};

use super::attack::{spell_uses, weapon_durability};
use super::info::stat_name;
use super::layout::{MAP_VIEW, SIDE_PANEL};
use super::skills::{skill_cost, skill_name, timed_text};
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::GlyphBuffer;
use crate::widgets::menu::{Menu, MenuItem};

/// What an attack is made with: the plain weapon, a Combat Art or a combat
/// active (one at most, `combat-arts.md`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Technique {
    /// A plain attack.
    #[default]
    Attack,
    /// A Combat Art.
    Art(ArtId),
    /// A combat active.
    Active(SkillId),
}

impl Technique {
    /// The attack on `target` with `with` (a weapon's slot, or an attack
    /// spell, which has no arts), using this.
    pub fn action(&self, target: UnitId, with: &Equipped) -> UnitAction {
        let (art, active) = match self {
            Technique::Attack => (None, None),
            Technique::Art(id) => (Some(id.clone()), None),
            Technique::Active(id) => (None, Some(id.clone())),
        };
        match with {
            Equipped::Weapon(slot) => UnitAction::Attack {
                target,
                slot: *slot,
                active,
                art,
            },
            Equipped::Spell(spell) => UnitAction::Cast {
                spell: spell.clone(),
                target: CastTarget::Unit(target),
                active,
            },
        }
    }

    /// Its name as the list and the forecast show it.
    pub fn name(&self, state: &BattleState) -> String {
        match self {
            Technique::Attack => "Attack".to_owned(),
            Technique::Art(id) => art_name(state, id),
            Technique::Active(id) => skill_name(state, id),
        }
    }
}

/// The name of art `id` (its id if the table lacks it).
pub fn art_name(state: &BattleState, id: &ArtId) -> String {
    state
        .arts()
        .get(id)
        .map_or_else(|| id.0.clone(), |a| a.name.clone())
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtChoice {
    /// What the line attacks with.
    pub technique: Technique,
    /// Why it can't be chosen now (`broken`, `out of range`); `None` if it
    /// can.
    pub reason: Option<String>,
}

/// Why a cost can't be paid, as the list shows it.
pub fn reason_text(error: &CostError) -> String {
    match error {
        CostError::WeaponBroken => "broken".to_owned(),
        CostError::NotEnoughUses { left } => format!("{left} uses left"),
        CostError::NoWeapon | CostError::WrongSource => "can't pay".to_owned(),
    }
}

/// The line for `technique`: usable if the core accepts the attack, dimmed
/// if only its cost stands in the way, not listed otherwise (it doesn't fit
/// this attack).
fn choice(
    state: &BattleState,
    unit: UnitId,
    dest: Pos,
    action: &UnitAction,
    technique: Technique,
) -> Option<ArtChoice> {
    let reason = match state.preview_attack(unit, dest, action) {
        Ok(_) => None,
        Err(CommandError::CannotPayArt { error, .. } | CommandError::CannotPay { error, .. }) => {
            Some(reason_text(&error))
        }
        Err(_) => return None,
    };
    Some(ArtChoice { technique, reason })
}

/// The list for `unit` attacking `target` from `dest` with `with` (the
/// weapon in a slot, or an attack spell): `Attack`, a weapon's arts (rank
/// arts lowest rank first, then the weapon's own), then the unit's combat
/// actives. `Attack` is always listed; it is dimmed (`out of range`) on a
/// target only an art or active reaches (Close Shot, Long Shot: 0426).
pub fn art_choices(
    state: &BattleState,
    unit: UnitId,
    dest: Pos,
    with: &Equipped,
    target: UnitId,
) -> Vec<ArtChoice> {
    let plain = Technique::Attack.action(target, with);
    let reason = match state.preview_attack(unit, dest, &plain) {
        Ok(_) => None,
        Err(CommandError::OutOfRange { .. }) => Some("out of range".to_owned()),
        Err(_) => Some("can't attack".to_owned()),
    };
    let mut out = vec![ArtChoice {
        technique: Technique::Attack,
        reason,
    }];
    let Some(u) = state.unit(unit) else {
        return out;
    };
    if let Equipped::Weapon(slot) = *with {
        let arts = u.arts_for(slot, state.classes(), state.items(), state.arts());
        for art in arts {
            let t = Technique::Art(art.id.clone());
            out.extend(choice(state, unit, dest, &t.action(target, with), t));
        }
    }
    let actives = u.usable_skills(state.classes(), state.skills());
    for skill in actives.into_iter().filter(|s| s.is_combat()) {
        let t = Technique::Active(skill.id.clone());
        out.extend(choice(state, unit, dest, &t.action(target, with), t));
    }
    out
}

/// What a line costs: `−4 dur` or `+1 use` (empty for `Attack`).
pub fn cost_label(state: &BattleState, technique: &Technique) -> String {
    let cost = match technique {
        Technique::Attack => None,
        Technique::Art(id) => state.arts().get(id).map(ArtDef::skill_cost),
        Technique::Active(id) => state.skills().get(id).and_then(skill_cost),
    };
    match cost {
        Some(SkillCost::Durability(n)) => format!("−{n} dur"),
        Some(SkillCost::ExtraSpellUse) => "+1 use".to_owned(),
        None => String::new(),
    }
}

/// Where a line comes from: the art's rank (`E`, `D`), `weapon` for a
/// weapon's own art, `active` for a class active (empty for `Attack`).
pub fn source_label(state: &BattleState, technique: &Technique) -> String {
    match technique {
        Technique::Attack => String::new(),
        Technique::Art(id) => match state.arts().get(id).and_then(|a| a.rank) {
            Some(rank) => format!("{rank:?}"),
            None => "weapon".to_owned(),
        },
        Technique::Active(_) => "active".to_owned(),
    }
}

/// Width of the name column.
const NAME_W: usize = 14;

/// The list as a menu: `Guard Break     −4 dur  D`, a dimmed line with its
/// reason after it, focused on `focus` (or the first line that can be
/// chosen).
pub fn art_menu(state: &BattleState, choices: &[ArtChoice], focus: usize) -> Menu {
    let items = choices
        .iter()
        .map(|c| {
            let name = c.technique.name(state);
            let cost = cost_label(state, &c.technique);
            let source = source_label(state, &c.technique);
            let label = format!("{name:<NAME_W$}  {cost:>6}  {source:<6}");
            match &c.reason {
                None => MenuItem::new(label),
                Some(why) => {
                    MenuItem::disabled(label).with_suffix(format!(" {why}"), UiColor::HpLow)
                }
            }
        })
        .collect();
    Menu::new(items).focused(focus)
}

/// A weapon's durability as its lines show it: `20/20`, or `broken` at 0.
pub fn durability_text(left: u32, max: u32) -> String {
    if left == 0 {
        "broken".to_owned()
    } else {
        format!("{left}/{max}")
    }
}

/// The list's box beside the forecast panel, on the half of the map away
/// from the row `unit_y` (the attacker's cell row), so it doesn't hide the
/// fight. `(w, h)` is the menu's size.
pub fn list_origin(unit_y: i32, (w, h): (i32, i32)) -> (i32, i32) {
    let x = (SIDE_PANEL.x - w).max(MAP_VIEW.x);
    let y = if unit_y < MAP_VIEW.y + MAP_VIEW.h / 2 {
        MAP_VIEW.y + MAP_VIEW.h - h
    } else {
        MAP_VIEW.y
    };
    (x, y)
}

/// Draws the list `menu` for `unit` attacking with `with`, with the
/// weapon's name and durability (`Iron Sword 20/20`) or the spell's name
/// and uses (`Fire 6/10`) in its top border.
pub fn draw_list(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    state: &BattleState,
    (unit, with): (UnitId, &Equipped),
    menu: &Menu,
    unit_y: i32,
) {
    let (x, y) = list_origin(unit_y, menu.size());
    menu.draw(palette, buf, x, y);
    let header = match with {
        Equipped::Weapon(slot) => weapon_durability(state, unit, *slot),
        Equipped::Spell(spell) => spell_uses(state, unit, spell),
    };
    let Some((name, left, max)) = header else {
        return;
    };
    let bg = palette.get(UiColor::PanelBg);
    let at = x + 2;
    let n = buf.print(
        at,
        y,
        &format!(" {name} "),
        palette.get(UiColor::TextHighlight),
        bg,
    );
    let color = if left == 0 {
        UiColor::HpLow
    } else {
        UiColor::Text
    };
    let dur = format!("{} ", durability_text(left, max));
    buf.print(at + i32::from(n), y, &dur, palette.get(color), bg);
}

/// The name of the art or active a boss or a green unit used in `events`
/// (`combat-arts.md`: "the forecast and playback show its name"), for the
/// playback's banner. `before` holds the units as they were before. The
/// player chose its own, so its units get none.
pub fn playback_banner(state: &BattleState, events: &[Event], before: &[Unit]) -> Option<String> {
    let not_player = |id: &UnitId| {
        before
            .iter()
            .any(|u| u.id == *id && u.faction != Faction::Player)
    };
    events.iter().find_map(|e| match e {
        Event::ArtUsed { unit, art, .. } if not_player(unit) => Some(art_name(state, art)),
        Event::SkillUsed { unit, skill } if not_player(unit) => Some(skill_name(state, skill)),
        _ => None,
    })
}

/// One of the art's non-number effects as text: `no counter`, `pierces`,
/// `pins: Mov −3`, `slows: Spd −3`, `stance: +20 avo`.
pub fn note_text(note: &ArtNote) -> String {
    match note {
        ArtNote::NoCounter => "no counter".to_owned(),
        ArtNote::Pierces => "pierces".to_owned(),
        ArtNote::Debuff(d) => {
            let verb = match d.stat {
                StatKind::Mov => "pins",
                StatKind::Spd => "slows",
                _ => "lowers",
            };
            format!("{verb}: {} −{}", stat_name(d.stat), d.amount)
        }
        ArtNote::Stance(mods) => format!("stance: {}", timed_text(mods)),
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{CombatMods, Debuff, TimedMods};

    use super::*;

    #[test]
    fn reasons_say_what_is_missing() {
        assert_eq!(reason_text(&CostError::WeaponBroken), "broken");
        assert_eq!(
            reason_text(&CostError::NotEnoughUses { left: 1 }),
            "1 uses left"
        );
        assert_eq!(reason_text(&CostError::NoWeapon), "can't pay");
        assert_eq!(reason_text(&CostError::WrongSource), "can't pay");
    }

    #[test]
    fn notes_read_as_the_design_writes_them() {
        assert_eq!(note_text(&ArtNote::NoCounter), "no counter");
        assert_eq!(note_text(&ArtNote::Pierces), "pierces");
        let pin = Debuff {
            stat: StatKind::Mov,
            amount: 3,
        };
        assert_eq!(note_text(&ArtNote::Debuff(pin)), "pins: Mov −3");
        let slow = Debuff {
            stat: StatKind::Spd,
            ..pin
        };
        assert_eq!(note_text(&ArtNote::Debuff(slow)), "slows: Spd −3");
        let other = Debuff {
            stat: StatKind::Def,
            ..pin
        };
        assert_eq!(note_text(&ArtNote::Debuff(other)), "lowers: Def −3");
        let stance = TimedMods {
            stats: vec![],
            combat: CombatMods {
                avoid: 20,
                ..CombatMods::default()
            },
        };
        assert_eq!(note_text(&ArtNote::Stance(stance)), "stance: +20 avo");
    }

    #[test]
    fn durability_reads_left_of_max_or_broken() {
        assert_eq!(durability_text(20, 20), "20/20");
        assert_eq!(durability_text(3, 20), "3/20");
        assert_eq!(durability_text(0, 20), "broken");
    }

    #[test]
    fn the_list_sits_beside_the_panel_away_from_the_attacker() {
        // An attacker in the top half: the list at the bottom of the map.
        assert_eq!(list_origin(3, (34, 6)), (SIDE_PANEL.x - 34, 24));
        // In the bottom half: at the top.
        assert_eq!(list_origin(20, (34, 6)), (SIDE_PANEL.x - 34, 0));
        assert_eq!(list_origin(15, (34, 6)).1, 0);
        // Never left of the map.
        assert_eq!(list_origin(3, (200, 6)).0, MAP_VIEW.x);
    }

    #[test]
    fn a_plain_attack_has_no_art_or_active() {
        let t = Technique::default();
        assert_eq!(t, Technique::Attack);
        assert_eq!(
            t.action(UnitId(4), &Equipped::Weapon(1)),
            UnitAction::Attack {
                target: UnitId(4),
                slot: 1,
                active: None,
                art: None,
            }
        );
        let art = Technique::Art(ArtId::new("guard_break"));
        assert!(matches!(
            art.action(UnitId(4), &Equipped::Weapon(0)),
            UnitAction::Attack {
                art: Some(_),
                active: None,
                ..
            }
        ));
        let active = Technique::Active(SkillId::new("keen_edge"));
        assert!(matches!(
            active.action(UnitId(4), &Equipped::Weapon(0)),
            UnitAction::Attack {
                art: None,
                active: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn with_a_spell_the_attack_is_a_cast_at_the_unit() {
        let fire = Equipped::Spell(trpg_core::SpellId::new("fire"));
        let cast = |active| UnitAction::Cast {
            spell: trpg_core::SpellId::new("fire"),
            target: CastTarget::Unit(UnitId(4)),
            active,
        };
        assert_eq!(Technique::Attack.action(UnitId(4), &fire), cast(None));
        let overcast = SkillId::new("overcast");
        assert_eq!(
            Technique::Active(overcast.clone()).action(UnitId(4), &fire),
            cast(Some(overcast))
        );
    }
}
