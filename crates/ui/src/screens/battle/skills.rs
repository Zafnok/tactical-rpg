//! Skills in the battle UI (ticket 0412): the text that describes a skill
//! (its cost and one-line effect), the combat actives the attack flow can
//! cycle through, and the `Skill` menu of non-combat actives with the
//! target mode of Shove. Legality is the core's: the attack forecast and
//! the commands refuse what a unit can't do, and this module only asks (a
//! command applied to a copy of the battle), so the player is never offered
//! a skill that would be refused (ADR-0004).

use trpg_core::{
    ActiveEffect, Area, BattleState, CombatMods, Command, Condition, PassiveEffect, Pos, SkillCost,
    SkillDef, SkillId, SkillKind, StatKind, StatValue, TimedEffect, TimedMods, UnitAction, UnitId,
    WeaponReq,
};

use super::attack::attack_with;
use super::info::stat_name;
use super::mode::Selection;
use crate::widgets::menu::{Menu, MenuItem};

/// A skill's cost as text: `3 dur` (weapon durability) or `+1 use` (one
/// more use of the spell).
pub fn cost_text(cost: SkillCost) -> String {
    match cost {
        SkillCost::Durability(n) => format!("{n} dur"),
        SkillCost::ExtraSpellUse => "+1 use".to_owned(),
    }
}

/// `+30 hit +10 crit`: the non-zero numbers and flags of `mods`.
pub fn mods_text(mods: &CombatMods) -> String {
    let numbers = [
        (mods.hit, "hit"),
        (mods.crit, "crit"),
        (mods.might, "might"),
        (mods.avoid, "avo"),
        (mods.attack_speed, "spd"),
        (mods.pierce, "pierce"),
    ];
    let mut parts: Vec<String> = numbers
        .iter()
        .filter(|(n, _)| *n != 0)
        .map(|(n, what)| format!("{n:+} {what}"))
        .collect();
    if mods.extra_strikes > 0 {
        parts.push(format!("+{} strike", mods.extra_strikes));
    }
    let flags = [
        (mods.single_strike, "1 strike only"),
        (mods.double_crit, "double crit"),
        (mods.ignore_terrain, "ignores terrain"),
        (mods.sword_followup.is_some(), "new follow-up"),
    ];
    parts.extend(
        flags
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, t)| (*t).to_owned()),
    );
    parts.join(" ")
}

/// `Def +5 Res +5`: the stat bonuses of `stats`.
pub fn stats_text(stats: &[(StatKind, StatValue)]) -> String {
    stats
        .iter()
        .map(|&(kind, n)| format!("{} {n:+}", stat_name(kind)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A timed effect's bonuses as text: its stats, then its combat numbers.
pub fn timed_text(mods: &TimedMods) -> String {
    [stats_text(&mods.stats), mods_text(&mods.combat)]
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// When a passive's bonus applies, as a short tail (empty for always).
fn condition_text(when: Condition) -> String {
    match when {
        Condition::Always => String::new(),
        Condition::WeaponKindEquipped(kind) => format!("with {kind:?}"),
        Condition::NotOwnPhase => "when attacked".to_owned(),
        Condition::HpAtMostHalf => "at half HP".to_owned(),
        Condition::MovedAtLeast(n) => format!("after {n}+ moves"),
        Condition::AgainstWeaponKind(kind) => format!("vs {kind:?}"),
    }
}

/// `text` and then `tail`, apart by a space unless the tail is empty.
fn then(text: String, tail: &str) -> String {
    if tail.is_empty() {
        text
    } else {
        format!("{text} {tail}")
    }
}

/// One passive effect as text, e.g. `+10 crit with Sword`.
fn passive_text(effect: &PassiveEffect) -> String {
    match effect {
        PassiveEffect::StatWhile { stat, amount, when } => then(
            format!("{} {amount:+}", stat_name(*stat)),
            &condition_text(*when),
        ),
        PassiveEffect::CombatMod { mods, when } => then(mods_text(mods), &condition_text(*when)),
        PassiveEffect::HealBonus(n) => format!("heals {n:+}"),
        PassiveEffect::SpellMight(n) => format!("spell might {n:+}"),
        PassiveEffect::PostActionMove { tiles, when } => {
            then(format!("move {tiles} after attack"), &condition_text(*when))
        }
        PassiveEffect::AllyAura { radius, mods } => {
            format!("ally r{radius}: {}", mods_text(mods))
        }
    }
}

/// An active's effect as text, e.g. `Sword: +30 hit +10 crit`.
fn active_text(effect: &ActiveEffect) -> String {
    match effect {
        ActiveEffect::Strike {
            with,
            mods,
            range,
            stance,
            post_move,
            drain,
        } => {
            let mut parts = vec![mods_text(mods)];
            if *range > 0 {
                parts.push(format!("+{range} range"));
            }
            if let Some(stance) = stance {
                parts.push(format!("stance {}", timed_text(&stance.mods)));
            }
            if *post_move > 0 {
                parts.push(format!("move {post_move} after"));
            }
            if *drain {
                parts.push("drains".to_owned());
            }
            let body = parts
                .into_iter()
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            match with {
                WeaponReq::Any => body,
                WeaponReq::Kind(kind) => format!("{kind:?}: {body}"),
                WeaponReq::Spell => format!("Spell: {body}"),
            }
        }
        ActiveEffect::Buff { area, mods } => {
            let who = match area {
                Area::Own => "self".to_owned(),
                Area::Allies { radius } => format!("ally r{radius}"),
            };
            format!("{who}: {}", timed_text(mods))
        }
        ActiveEffect::Heal { radius, power } => {
            format!("heals ally r{radius} (Mag {power:+})")
        }
        ActiveEffect::Push { collision } => format!("push 1 tile, {collision} on impact"),
    }
}

/// A skill's effect in a few words.
pub fn effect_text(skill: &SkillDef) -> String {
    match &skill.kind {
        SkillKind::Passive(effects) => effects
            .iter()
            .map(passive_text)
            .collect::<Vec<_>>()
            .join("; "),
        SkillKind::Active { effect, .. } => active_text(effect),
    }
}

/// An active's cost, `None` for a passive.
pub fn skill_cost(skill: &SkillDef) -> Option<SkillCost> {
    match skill.kind {
        SkillKind::Active { cost, .. } => Some(cost),
        SkillKind::Passive(_) => None,
    }
}

/// The name of skill `id` (its id if the table lacks it).
pub fn skill_name(state: &BattleState, id: &SkillId) -> String {
    state
        .skills()
        .get(id)
        .map_or_else(|| id.0.clone(), |s| s.name.clone())
}

/// When a timed effect ends, e.g. `until Player phase`.
pub fn until_text(effect: &TimedEffect) -> String {
    format!("until {:?} phase", effect.until)
}

/// The combat actives `unit` could use attacking `target` from `dest` with
/// the weapon in `slot`, in the order of its skills: only those the
/// forecast accepts, so one that can't be paid for is skipped.
pub fn combat_actives(
    state: &BattleState,
    unit: UnitId,
    dest: Pos,
    slot: usize,
    target: UnitId,
) -> Vec<SkillId> {
    let Some(u) = state.unit(unit) else {
        return vec![];
    };
    u.usable_skills(state.classes(), state.skills())
        .into_iter()
        .filter(|s| s.is_combat())
        .filter(|s| {
            let action = attack_with(target, slot, Some(s.id.clone()));
            state.preview_attack(unit, dest, &action).is_ok()
        })
        .map(|s| s.id.clone())
        .collect()
}

/// The non-combat actives `unit` knows, in the order of its skills.
fn known_non_combat(state: &BattleState, unit: UnitId) -> Vec<&SkillDef> {
    let Some(u) = state.unit(unit) else {
        return vec![];
    };
    u.usable_skills(state.classes(), state.skills())
        .into_iter()
        .filter(|s| s.is_active() && !s.is_combat())
        .collect()
}

/// Whether the action menu has a `Skill` entry: `unit` knows a non-combat
/// active (whether or not it can use it now).
pub fn has_skill_menu(state: &BattleState, unit: UnitId) -> bool {
    !known_non_combat(state, unit).is_empty()
}

/// One line of the `Skill` menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillChoice {
    /// The skill.
    pub skill: SkillId,
    /// Whether it is used on a chosen unit (Shove) rather than on no one
    /// in particular.
    pub needs_target: bool,
    /// Who it can be used on (empty when it needs no target).
    pub targets: Vec<UnitId>,
    /// Whether the core would accept it now; if not, the line is dimmed.
    pub usable: bool,
}

/// The command that uses `skill` from `sel`'s path end on `target`.
fn use_command(sel: &Selection, skill: &SkillId, target: Option<UnitId>) -> Command {
    Command::Act {
        unit: sel.unit,
        dest: sel.dest(),
        action: UnitAction::UseSkill {
            skill: skill.clone(),
            target,
        },
    }
}

/// Whether the core accepts `cmd` (applied to a copy).
fn accepted(state: &BattleState, cmd: &Command) -> bool {
    state.clone().apply(cmd).is_ok()
}

/// The `Skill` menu's lines for `sel`'s unit.
pub fn skill_choices(state: &BattleState, sel: &Selection) -> Vec<SkillChoice> {
    known_non_combat(state, sel.unit)
        .into_iter()
        .map(|def| {
            let needs_target = matches!(
                def.kind,
                SkillKind::Active {
                    effect: ActiveEffect::Push { .. },
                    ..
                }
            );
            let (targets, usable) = if needs_target {
                let mut found: Vec<(Pos, UnitId)> = state
                    .units()
                    .iter()
                    .filter(|u| Pos::manhattan(sel.dest(), u.pos) == 1)
                    .filter(|u| accepted(state, &use_command(sel, &def.id, Some(u.id))))
                    .map(|u| (u.pos, u.id))
                    .collect();
                found.sort_by_key(|(p, _)| (p.y, p.x));
                let targets: Vec<UnitId> = found.into_iter().map(|(_, id)| id).collect();
                let usable = !targets.is_empty();
                (targets, usable)
            } else {
                let usable = accepted(state, &use_command(sel, &def.id, None));
                (vec![], usable)
            };
            SkillChoice {
                skill: def.id.clone(),
                needs_target,
                targets,
                usable,
            }
        })
        .collect()
}

/// Whether `Skill` is enabled: some skill can be used now.
pub fn can_use_skill(choices: &[SkillChoice]) -> bool {
    choices.iter().any(|c| c.usable)
}

/// The equipped weapon's `durability left/max`, as text.
fn equipped_durability(state: &BattleState, unit: UnitId) -> Option<(u32, u32)> {
    let u = state.unit(unit)?;
    let copy = u.loadout.weapon(u.loadout.equipped_slot()?)?;
    let def = state.items().weapon(&copy.def)?;
    Some((copy.durability_left, def.durability))
}

/// The `Skill` menu: `Brace  3 dur  Wpn 20/20`, unusable lines dimmed.
pub fn skill_menu(state: &BattleState, unit: UnitId, choices: &[SkillChoice]) -> Menu {
    let name_w = choices
        .iter()
        .map(|c| skill_name(state, &c.skill).chars().count())
        .max()
        .unwrap_or(0);
    let weapon = equipped_durability(state, unit)
        .map_or_else(String::new, |(left, max)| format!("  Wpn {left}/{max}"));
    let items = choices
        .iter()
        .map(|c| {
            let cost = state
                .skills()
                .get(&c.skill)
                .and_then(skill_cost)
                .map(cost_text)
                .unwrap_or_default();
            let text = format!(
                "{:<name_w$}  {cost:>6}{weapon}",
                skill_name(state, &c.skill)
            );
            if c.usable {
                MenuItem::new(text)
            } else {
                MenuItem::disabled(text)
            }
        })
        .collect();
    Menu::new(items)
}

/// Picking who a skill (Shove) is used on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTargeting {
    /// The unit using the skill and its path (it stands at the path's end).
    pub sel: Selection,
    /// The `Skill` menu it was opened from (Cancel goes back to it).
    pub menu: Menu,
    /// The menu's lines.
    pub choices: Vec<SkillChoice>,
    /// Which line is being used.
    pub choice: usize,
    /// The target under the cursor, an index of the line's targets.
    pub index: usize,
}

impl SkillTargeting {
    /// Targeting for `choices[choice]`. `None` if it has no target.
    pub fn new(
        sel: Selection,
        menu: Menu,
        choices: Vec<SkillChoice>,
        choice: usize,
    ) -> Option<Self> {
        if choices.get(choice)?.targets.is_empty() {
            return None;
        }
        Some(Self {
            sel,
            menu,
            choices,
            choice,
            index: 0,
        })
    }

    /// The line being used.
    fn used(&self) -> &SkillChoice {
        // `choice` is valid and its targets are never empty ([`Self::new`]).
        &self.choices[self.choice]
    }

    /// The skill being used.
    pub fn skill(&self) -> &SkillId {
        &self.used().skill
    }

    /// The target under the cursor.
    pub fn target(&self) -> UnitId {
        self.used().targets[self.index]
    }

    /// Who the skill can be used on.
    pub fn targets(&self) -> &[UnitId] {
        &self.used().targets
    }

    /// Moves to the next target (or the previous one), wrapping.
    pub fn cycle(&mut self, forward: bool) {
        let n = self.used().targets.len();
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
    }

    /// The command that uses the skill.
    pub fn command(&self) -> Command {
        use_command(&self.sel, self.skill(), Some(self.target()))
    }

    /// The preview line, e.g. `Shove on Brigand (20 → 17)`.
    pub fn preview(&self, state: &BattleState) -> String {
        let name = skill_name(state, self.skill());
        let who = state
            .unit(self.target())
            .map_or(String::new(), |u| format!(" on {}", u.name));
        let cost = durability_change(state, self.sel.unit, self.skill());
        format!("{name}{who}{cost}")
    }
}

/// ` (20 → 17)`: the equipped weapon's durability before and after paying
/// `skill`, empty if it costs none.
pub fn durability_change(state: &BattleState, unit: UnitId, skill: &SkillId) -> String {
    let cost = state.skills().get(skill).and_then(skill_cost);
    match (cost, equipped_durability(state, unit)) {
        (Some(SkillCost::Durability(n)), Some((left, _))) => {
            format!(" ({left} → {})", left.saturating_sub(n))
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::SkillTable;

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::testing::{battle_with, quick_units};

    fn def<'a>(skills: &'a SkillTable, id: &str) -> &'a SkillDef {
        skills.get(&SkillId::new(id)).unwrap()
    }

    #[test]
    fn costs_read_as_durability_or_spell_uses() {
        assert_eq!(cost_text(SkillCost::Durability(3)), "3 dur");
        assert_eq!(cost_text(SkillCost::ExtraSpellUse), "+1 use");
    }

    #[test]
    fn effects_read_as_one_line() {
        let c = ctx();
        let skills = &c.content.skills;
        assert_eq!(
            effect_text(def(skills, "keen_edge")),
            "Sword: +30 hit +10 crit"
        );
        assert_eq!(effect_text(def(skills, "brace")), "self: Def +5 Res +5");
        assert_eq!(
            effect_text(def(skills, "sword_focus_1")),
            "+10 crit with Sword"
        );
        assert_eq!(
            effect_text(def(skills, "steadfast_1")),
            "Def +2 when attacked"
        );
        assert_eq!(
            effect_text(def(skills, "shove")),
            "push 1 tile, 5 on impact"
        );
        assert_eq!(skill_cost(def(skills, "sword_focus_1")), None);
    }

    #[test]
    fn range_and_moves_after_show_only_when_set() {
        let c = ctx();
        let skills = &c.content.skills;
        assert_eq!(effect_text(def(skills, "long_shot")), "Bow: +2 range");
        assert_eq!(effect_text(def(skills, "vault")), "Bow: move 1 after");
        assert!(!effect_text(def(skills, "keen_edge")).contains("range"));
        assert!(!effect_text(def(skills, "keen_edge")).contains("after"));
    }

    #[test]
    fn skill_targets_cycle_both_ways_and_wrap() {
        let c = ctx();
        let state = battle_with(
            &c,
            quick_units(&c).0,
            quick_units(&c).1,
            trpg_core::Objective::Rout { turn_limit: None },
        );
        let sel = Selection::new(&state, UnitId(1)).unwrap();
        let choice = SkillChoice {
            skill: SkillId::new("shove"),
            needs_target: true,
            targets: vec![UnitId(4), UnitId(5), UnitId(6)],
            usable: true,
        };
        let menu = Menu::new(vec![]);
        let mut t = SkillTargeting::new(sel, menu, vec![choice], 0).unwrap();
        assert_eq!(t.targets(), [UnitId(4), UnitId(5), UnitId(6)]);
        let mut seen = vec![t.target()];
        for _ in 0..3 {
            t.cycle(true);
            seen.push(t.target());
        }
        assert_eq!(seen, [UnitId(4), UnitId(5), UnitId(6), UnitId(4)]);
        t.cycle(false);
        assert_eq!(t.target(), UnitId(6));
        t.cycle(false);
        assert_eq!(t.target(), UnitId(5));
        assert_eq!(
            t.command(),
            use_command(&t.sel, &SkillId::new("shove"), Some(UnitId(5)))
        );
    }

    #[test]
    fn every_skill_has_a_short_effect_line() {
        let c = ctx();
        for skill in c.content.skills.skills.values() {
            let text = effect_text(skill);
            assert!(!text.is_empty(), "{} has no text", skill.id.0);
        }
    }

    #[test]
    fn mods_and_stats_list_only_what_is_set() {
        assert_eq!(mods_text(&CombatMods::default()), "");
        let mods = CombatMods {
            avoid: -3,
            extra_strikes: 1,
            double_crit: true,
            ..CombatMods::default()
        };
        assert_eq!(mods_text(&mods), "-3 avo +1 strike double crit");
        assert_eq!(
            stats_text(&[(StatKind::Def, 5), (StatKind::Res, -2)]),
            "Def +5 Res -2"
        );
    }
}
