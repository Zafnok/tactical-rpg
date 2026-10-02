//! The `Magic` menu (ticket 0410, `docs/design/magic.md`): a unit's spell
//! list with what each spell can be cast on from where the unit stands, and
//! the cast target mode. One cursor steps through everything the chosen
//! spell can target (Nick): an enemy shows the attack forecast (0404's
//! [`Targeting`], with the unit's spell actives as its list), a hurt ally
//! the heal's `HP 10 → 25`, an empty tile the terrain change (`Forest →
//! Burning (1 round)`). Every target is one the core accepts
//! ([`BattleState::check`]), so the UI never decides what is legal
//! (ADR-0004).

use trpg_core::{
    BattleState, CastTarget, Command, EffectDuration, Equipped, Pos, SpellDef, SpellId, SpellKind,
    TerrainId, UnitAction, UnitId,
};

use super::attack::Targeting;
use super::info::range_text;
use super::mode::Selection;
use crate::widgets::menu::{Menu, MenuItem};

/// One line of the spell list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellChoice {
    /// The spell.
    pub spell: SpellId,
    /// What it can be cast on from the destination, in `(y, x)` order of
    /// their tiles: enemies (an attack spell), hurt allies (a heal), empty
    /// tiles (a terrain effect). Empty: the line is dimmed.
    pub targets: Vec<CastTarget>,
}

/// Casting `spell` on `target`, with no active.
pub fn cast(spell: &SpellId, target: CastTarget) -> UnitAction {
    UnitAction::Cast {
        spell: spell.clone(),
        target,
        active: None,
    }
}

/// The tile `target` is on: a unit's, or the tile itself.
pub fn target_pos(state: &BattleState, target: CastTarget) -> Option<Pos> {
    match target {
        CastTarget::Unit(id) => state.unit(id).map(|u| u.pos),
        CastTarget::Tile(pos) => Some(pos),
    }
}

/// What `unit` could cast `spell` on from `dest`: the units and, for a
/// spell with a terrain effect, the tiles in its range that the core
/// accepts, in `(y, x)` order of their tiles.
pub fn cast_targets(
    state: &BattleState,
    unit: UnitId,
    dest: Pos,
    spell: &SpellDef,
) -> Vec<CastTarget> {
    let legal = |target: CastTarget| {
        let action = cast(&spell.id, target);
        state.check(&Command::Act { unit, dest, action }).is_ok()
    };
    let near = |pos: Pos| spell.in_range(Pos::manhattan(dest, pos));
    let units = state
        .units()
        .iter()
        .map(|u| (u.pos, CastTarget::Unit(u.id)));
    let tiles = state
        .map()
        .tiles
        .positions()
        .filter(|_| spell.terrain_effect.is_some())
        .map(|pos| (pos, CastTarget::Tile(pos)));
    let mut found: Vec<(Pos, CastTarget)> = units
        .chain(tiles)
        .filter(|&(pos, target)| near(pos) && legal(target))
        .collect();
    found.sort_by_key(|(p, _)| (p.y, p.x));
    found.into_iter().map(|(_, target)| target).collect()
}

/// The spells `unit` has learned, in id order, with what each could be cast
/// on from `dest`.
pub fn spell_choices(state: &BattleState, unit: UnitId, dest: Pos) -> Vec<SpellChoice> {
    let Some(u) = state.unit(unit) else {
        return vec![];
    };
    u.learned
        .iter()
        .filter_map(|id| state.spells().get(id))
        .map(|def| SpellChoice {
            spell: def.id.clone(),
            targets: cast_targets(state, unit, dest, def),
        })
        .collect()
}

/// Whether the action menu lists `Magic` for `unit`: it knows a spell.
pub fn knows_spells(state: &BattleState, unit: UnitId) -> bool {
    state
        .unit(unit)
        .is_some_and(|u| u.learned.iter().any(|id| state.spells().get(id).is_some()))
}

/// Whether `Magic` is enabled: some spell has something to be cast on.
pub fn can_cast(choices: &[SpellChoice]) -> bool {
    choices.iter().any(|c| !c.targets.is_empty())
}

/// Whether some attack spell reaches an enemy (the action menu then opens
/// on `Magic`, as it opens on `Attack` when a weapon does).
pub fn reaches_an_enemy(state: &BattleState, choices: &[SpellChoice]) -> bool {
    choices.iter().any(|c| {
        let attack = state
            .spells()
            .get(&c.spell)
            .is_some_and(SpellDef::is_attack);
        attack && c.targets.iter().any(|t| matches!(t, CastTarget::Unit(_)))
    })
}

/// A spell's line: name, uses left of its uses per battle, and its numbers,
/// e.g. `Fire    6/10  Mt5 Hit90 Rng1-2` or `Heal    8/8   HP+10 Rng1`.
pub fn spell_label(def: &SpellDef, left: u8, name_w: usize) -> String {
    let numbers = match &def.kind {
        SpellKind::Attack { might, hit, .. } => format!("Mt{might} Hit{hit}"),
        SpellKind::Heal { heal_power } => format!("HP+{heal_power}"),
    };
    format!(
        "{:<name_w$}  {left:>2}/{:<2}  {numbers} Rng{}",
        def.name,
        def.uses,
        range_text(def.min_range, def.max_range)
    )
}

/// The spell list: one line per choice, dimmed when it has nothing to be
/// cast on (always so at 0 uses), focused on the equipped spell if it can
/// be cast.
pub fn spell_menu(state: &BattleState, unit: UnitId, choices: &[SpellChoice]) -> Menu {
    let defs = || {
        choices
            .iter()
            .filter_map(|c| Some((c, state.spells().get(&c.spell)?)))
    };
    let name_w = defs().map(|(_, d)| d.name.chars().count()).max();
    let caster = state.unit(unit);
    let items = defs()
        .map(|(c, def)| {
            let left = caster.map_or(0, |u| u.spells.uses_left(&c.spell));
            let label = spell_label(def, left, name_w.unwrap_or(0));
            if c.targets.is_empty() {
                MenuItem::disabled(label)
            } else {
                MenuItem::new(label)
            }
        })
        .collect();
    let equipped = caster.and_then(|u| u.loadout.equipped_spell());
    let focus = choices.iter().position(|c| Some(&c.spell) == equipped);
    let menu = Menu::new(items);
    match focus {
        Some(i) => menu.focused(i),
        None => menu,
    }
}

/// Picking what a spell is cast on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastTargeting {
    /// The caster and its path (it stands at the path's end).
    pub sel: Selection,
    /// The spell list it was opened from (Cancel goes back to it).
    pub menu: Menu,
    /// The spell list's lines.
    pub choices: Vec<SpellChoice>,
    /// Which line is being cast.
    pub chosen: usize,
    /// The target under the cursor, an index of the line's targets.
    pub index: usize,
    /// The forecast (and list of spell actives) against the units an attack
    /// spell reaches; shown while the cursor is on one of them.
    attack: Option<Targeting>,
}

impl CastTargeting {
    /// Targeting for `choices[chosen]`, on its first target. `None` if it
    /// has none.
    pub fn new(
        state: &BattleState,
        sel: Selection,
        menu: Menu,
        choices: Vec<SpellChoice>,
        chosen: usize,
    ) -> Option<Self> {
        let choice = choices.get(chosen)?;
        let first = *choice.targets.first()?;
        let enemies: Vec<UnitId> = choice
            .targets
            .iter()
            .filter_map(|t| match t {
                CastTarget::Unit(id) => Some(*id),
                CastTarget::Tile(_) => None,
            })
            .collect();
        // No forecast can be made of a heal: its units get the HP line.
        let with = Equipped::Spell(choice.spell.clone());
        let mut attack = Targeting::with(state, sel.clone(), with, enemies, None);
        if let (Some(attack), CastTarget::Unit(id)) = (&mut attack, first) {
            attack.aim(id, state);
        }
        Some(Self {
            sel,
            menu,
            choices,
            chosen,
            index: 0,
            attack,
        })
    }

    /// The line being cast.
    fn used(&self) -> &SpellChoice {
        // `chosen` is valid and its targets are never empty ([`Self::new`]).
        &self.choices[self.chosen]
    }

    /// The spell being cast.
    pub fn spell(&self) -> &SpellId {
        &self.used().spell
    }

    /// The target under the cursor.
    pub fn target(&self) -> CastTarget {
        self.used().targets[self.index]
    }

    /// Everything the spell can be cast on.
    pub fn targets(&self) -> &[CastTarget] {
        &self.used().targets
    }

    /// The attack forecast, while the cursor is on a unit an attack spell
    /// hits.
    pub fn forecast(&self) -> Option<&Targeting> {
        match self.target() {
            CastTarget::Unit(_) => self.attack.as_ref(),
            CastTarget::Tile(_) => None,
        }
    }

    /// Moves to the next target (or the previous one), wrapping; on an
    /// enemy, the forecast follows.
    pub fn cycle(&mut self, forward: bool, state: &BattleState) {
        let n = self.used().targets.len();
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
        if let (CastTarget::Unit(id), Some(attack)) = (self.target(), &mut self.attack) {
            attack.aim(id, state);
        }
    }

    /// Moves the focus of the forecast's list of spell actives (see
    /// [`Targeting::move_list`]); nothing without a forecast.
    pub fn move_list(&mut self, down: bool, state: &BattleState) {
        if let (CastTarget::Unit(_), Some(attack)) = (self.target(), &mut self.attack) {
            attack.move_list(down, state);
        }
    }

    /// The command that casts the spell.
    pub fn command(&self) -> Command {
        match self.forecast() {
            Some(attack) => attack.command(),
            None => Command::Act {
                unit: self.sel.unit,
                dest: self.sel.dest(),
                action: cast(self.spell(), self.target()),
            },
        }
    }

    /// The tiles the spell can be cast on, each with the terrain it would
    /// become.
    pub fn tile_changes(&self, state: &BattleState) -> Vec<(Pos, TerrainId)> {
        let effect = state
            .spells()
            .get(self.spell())
            .and_then(|def| def.terrain_effect.as_ref());
        let Some(effect) = effect else {
            return vec![];
        };
        self.targets()
            .iter()
            .filter_map(|t| match t {
                CastTarget::Tile(pos) => Some((*pos, effect.to)),
                CastTarget::Unit(_) => None,
            })
            .collect()
    }

    /// The preview line of a heal (`Heal on Rex: HP 10 → 25`) or a tile
    /// cast (`Forest → Burning (1 round)`, `Sea → Ice`); none on an enemy,
    /// which has the forecast.
    pub fn preview(&self, state: &BattleState) -> Option<String> {
        let def = state.spells().get(self.spell())?;
        match self.target() {
            CastTarget::Unit(id) => {
                let target = state.unit(id)?;
                let action = cast(self.spell(), self.target());
                let gain = state.preview_heal(self.sel.unit, self.sel.dest(), &action)?;
                Some(format!(
                    "{} on {}: HP {} → {}",
                    def.name,
                    target.name,
                    target.hp,
                    target.hp + gain
                ))
            }
            CastTarget::Tile(pos) => {
                let effect = def.terrain_effect.as_ref()?;
                let name = |id| state.terrain().get(id).map(|t| t.name.as_str());
                let from = name(*state.map().tiles.get(pos)?)?;
                let to = name(effect.to)?;
                let lasts = match effect.lasts {
                    EffectDuration::Permanent => "",
                    EffectDuration::UntilCastersNextPhase { .. } => " (1 round)",
                };
                Some(format!("{from} → {to}{lasts}"))
            }
        }
    }
}
