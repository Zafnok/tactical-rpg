//! Small scripted battles for this crate's tests: generic swordsmen on the
//! plain row (`y = 4`, `x` 0 to 8) of the game's test map, with the game's
//! own tables.

use std::sync::LazyLock;

use trpg_content::Content;
use trpg_core::{
    BattlePack, BattleSetup, BattleState, ClassId, Command, Faction, GameMode, ItemId, LoadoutDef,
    Objective, Phase, Pos, Stock, Unit, UnitAction, UnitId,
};

use crate::BattleMeasures;

static CONTENT: LazyLock<Content> =
    LazyLock::new(|| trpg_content::load_embedded().unwrap_or_else(|e| panic!("content: {e}")));

/// The game's content.
pub(crate) fn content() -> &'static Content {
    &CONTENT
}

fn tile(x: i32) -> Pos {
    Pos::new(x, 4)
}

fn swordsman(id: u32, faction: Faction, x: i32) -> Unit {
    let content = content();
    let loadout = LoadoutDef {
        weapons: vec![ItemId::new("iron_sword")],
        ..LoadoutDef::default()
    };
    let class = ClassId("swordsman".into());
    Unit::generic(UnitId(id), &class, &content.classes, 1, faction, tile(x))
        .unwrap_or_else(|e| panic!("swordsman: {e}"))
        .with_loadout(&loadout, &content.classes, &content.items)
        .unwrap_or_else(|e| panic!("loadout: {e}"))
}

/// A player swordsman at `x`.
pub(crate) fn player(id: u32, x: i32) -> Unit {
    swordsman(id, Faction::Player, x)
}

/// An enemy swordsman at `x`.
pub(crate) fn enemy(id: u32, x: i32) -> Unit {
    swordsman(id, Faction::Enemy, x)
}

/// `unit`, never missing and felling anything it strikes.
pub(crate) fn rigged(mut unit: Unit) -> Unit {
    unit.stats.dex = 99;
    unit.stats.str = 99;
    unit
}

/// `unit` at 1 HP.
pub(crate) fn weak(mut unit: Unit) -> Unit {
    unit.hp = 1;
    unit
}

/// A rout of `units`, with a potion, an elixir and a potion in the pack.
pub(crate) fn start(units: Vec<Unit>) -> BattleState {
    let content = content();
    let tables = content.tables();
    let pack = ["potion", "elixir", "potion"];
    let (state, _) = BattleState::new(BattleSetup {
        map: content.maps["test_small"].map.clone(),
        terrain: tables.terrain,
        classes: tables.classes,
        items: tables.items,
        spells: tables.spells,
        skills: tables.skills,
        arts: tables.arts,
        pack: BattlePack {
            items: pack.iter().map(|i| ItemId::new(i)).collect(),
            cap: pack.len(),
        },
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 0,
        seed: 1,
        triggers: vec![],
        battle_notes: vec![],
        mode: GameMode::Classic,
        battle_notes: vec![],
    });
    state
}

/// Attack `target` with the first weapon.
pub(crate) fn attack(target: u32) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: None,
        art: None,
    }
}

/// A battle played by hand, every command observed.
pub(crate) struct Scripted {
    pub(crate) state: BattleState,
    pub(crate) measures: BattleMeasures,
}

impl Scripted {
    pub(crate) fn new(state: BattleState) -> Self {
        let measures = BattleMeasures::new(&state);
        Self { state, measures }
    }

    /// Applies `command`, which must be accepted.
    pub(crate) fn step(&mut self, command: &Command) {
        let events = self
            .state
            .apply(command)
            .unwrap_or_else(|e| panic!("{command:?}: {e}"));
        self.measures.observe(&self.state, &events);
    }

    /// Unit `id` does `action` at `x`.
    pub(crate) fn act(&mut self, id: u32, x: i32, action: UnitAction) {
        self.step(&Command::Act {
            unit: UnitId(id),
            dest: tile(x),
            action,
        });
    }

    pub(crate) fn end_phase(&mut self) {
        self.step(&Command::EndPhase);
    }

    /// Ends phases, nobody acting, until the next Player phase.
    pub(crate) fn next_player_phase(&mut self) {
        self.end_phase();
        while self.state.phase() != Phase::Player {
            self.end_phase();
        }
    }
}
