//! Scripted tests of the Preparations screen (ticket 0408) through the real
//! game: the debug Quick Battle (`assets/battles/quick.ron`) opens it
//! first, with a stock of spare gear, six Potions and two Elixirs, and a
//! scout left out of the battle who carries a Steel Bow.

use trpg_core::{BattleState, ItemId, UnitId};
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// The lord's unit (the first slot).
const LORD: UnitId = UnitId(1);

/// From the title: Quick Battle, which opens Preparations.
fn preparations() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "preparations"]);
    h
}

/// The battle on screen.
fn battle(h: &Harness) -> BattleState {
    let battle = h.flow().and_then(|f| f.battle());
    battle
        .unwrap_or_else(|| panic!("no battle"))
        .state()
        .clone()
}

/// The lord's weapons in the battle on screen, by slot.
fn lord_weapons(state: &BattleState) -> Vec<String> {
    let lord = state.unit(LORD).unwrap_or_else(|| panic!("no lord"));
    let weapons = lord.loadout.weapons.iter().flatten();
    weapons.map(|w| w.def.0.clone()).collect()
}

fn potion() -> ItemId {
    ItemId::new("potion")
}

/// Whether some row of the screen contains `text`.
fn shows(h: &Harness, text: &str) -> bool {
    let buf = h.game().buffer();
    (0..32).any(|y| {
        (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect::<String>()
            .contains(text)
    })
}

/// Loadouts → the lord → his empty third slot → the Iron Sword (the last
/// of the stock's three weapons; the others he can't use) → back to the
/// tabs; then Pack → two Potions (below the Elixirs) → back to the tabs.
const SET_UP: &str = "f f Down Down f Up f d d Right f Down f f d";

/// Acceptance: move a weapon from the stock into a slot, add 2 Potions to
/// the pack, Fight! → the battle starts with that loadout and pack.
#[test]
fn the_battle_starts_with_the_loadout_and_pack_set_up() {
    let mut h = preparations();
    h.keys(SET_UP);
    assert!(shows(&h, "Pack 2/6"), "{}", h.snapshot());
    // From the Pack tab, Right is `Fight!`.
    h.keys("Right f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let state = battle(&h);
    assert_eq!(
        lord_weapons(&state),
        ["iron_sword", "steel_sword", "iron_sword"]
    );
    assert_eq!(state.pack().items, [potion(), potion()]);
    assert_eq!(state.pack().cap, 6);
    // What wasn't brought stays in the stock.
    assert_eq!(state.stock().count(&potion()), 4);
    assert_eq!(state.stock().count(&ItemId::new("elixir")), 2);
    assert_eq!(state.stock().weapons.len(), 2);
}

/// Acceptance: the pack can never exceed the cap; unusable items can't be
/// equipped.
#[test]
fn the_cap_and_unusable_items_hold() {
    let mut h = preparations();
    // The lord's third slot, on the Steel Spear: refused, with the reason.
    h.keys("f f Down Down f");
    assert!(shows(&h, "Steel Spear     can't use spears"));
    h.keys("f");
    assert!(shows(&h, "Test Lord can't use spears."), "{}", h.snapshot());
    // Every consumable there is: two Elixirs, then Potions until full.
    h.keys("d d d Right f f f f f f f f f");
    assert!(shows(&h, "Pack 6/6"), "{}", h.snapshot());
    assert!(shows(&h, "The pack is full (6/6)."));
    h.keys("d Right f");
    let state = battle(&h);
    assert_eq!(lord_weapons(&state), ["iron_sword", "steel_sword"]);
    assert_eq!(state.pack().items.len(), 6);
    assert_eq!(state.stock().count(&potion()), 2);
}

/// Nick (0408): restarting the battle goes back to Preparations, as the
/// player left them, and the battle then starts the same again.
#[test]
fn restart_battle_goes_back_to_preparations_as_they_were_left() {
    let mut h = preparations();
    h.keys(SET_UP);
    h.keys("Right f");
    let start = battle(&h);
    // The lord waits where he stands, then the map menu's Restart Battle
    // (after Units and Objective) and its confirm.
    h.keys("f f f d Down Down f");
    assert!(
        shows(&h, "Restart the battle from turn 1?"),
        "{}",
        h.snapshot()
    );
    h.keys("f");
    assert_eq!(h.screens(), ["title", "preparations"]);
    let prep = h.flow().and_then(|f| f.preparations());
    let prep = prep.unwrap_or_else(|| panic!("no preparations"));
    assert_eq!(prep.packed(), [(potion(), 2)]);
    assert_eq!(prep.setup().stock.weapons.len(), 2);
    // One more Potion this time.
    h.keys("Right f Down f d Right f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let again = battle(&h);
    assert_eq!(again.pack().items, [potion(), potion(), potion()]);
    assert_eq!(lord_weapons(&again), lord_weapons(&start));
    assert_eq!(again.turn(), 1);
    let charges = h.flow().and_then(|f| f.battle());
    assert_eq!(charges.map(|b| b.history().charges_left()), Some(3));
}

/// The Quick Battle's Preparations can be left: back to the title.
#[test]
fn leaving_the_quick_battles_preparations_returns_to_the_title() {
    let mut h = preparations();
    h.keys("d");
    assert!(shows(&h, "Leave preparations?"), "{}", h.snapshot());
    h.keys("d");
    assert!(!shows(&h, "Leave preparations?"));
    assert_eq!(h.screens(), ["title", "preparations"]);
    h.keys("d f");
    assert_eq!(h.screens(), ["title"]);
}

/// The screen works with the player's own keys and a controller.
#[test]
fn rebound_keys_and_a_controller_work() {
    let mut h = Harness::with_layout(Layout::LeftHanded);
    // Left-handed: `s` is down, `j` confirms, `a` is left.
    h.keys("s j");
    assert_eq!(h.screens(), ["title", "preparations"]);
    assert!(
        shows(&h, "wasd choose · j open · k leave"),
        "{}",
        h.snapshot()
    );
    h.pad("DpadRight South South DpadRight East");
    let prep = h.flow().and_then(|f| f.preparations());
    let prep = prep.unwrap_or_else(|| panic!("no preparations"));
    assert_eq!(prep.packed(), [(ItemId::new("elixir"), 1)]);
    h.keys("d j");
    assert_eq!(h.screens(), ["title", "battle"]);
}

/// Nick (0408): two archers, only one in the battle, and the other has the
/// better bow: trade it on the Preparations screen. The trade holds through
/// the battle and a restart, and the scout is left without it.
#[test]
fn the_benched_scouts_bow_goes_to_the_archer() {
    let mut h = preparations();
    // The scout (last in the list): her only weapon back to the stock.
    h.keys("f Up f f f");
    // The archer (above her): his second slot takes it (the last stock row).
    h.keys("d Up f Down f Up f");
    assert!(
        shows(&h, "Weapon     Steel Bow       25/25"),
        "{}",
        h.snapshot()
    );
    h.keys("d d Left f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let state = battle(&h);
    let archer = state.unit(UnitId(3)).unwrap_or_else(|| panic!("no archer"));
    let bows: Vec<&str> = archer
        .loadout
        .weapons
        .iter()
        .flatten()
        .map(|w| w.def.0.as_str())
        .collect();
    assert_eq!(bows, ["iron_bow", "steel_bow"]);
    // The scout isn't in the battle, and in the army she has no bow now.
    assert_eq!(state.units().len(), 6);
    let scout = |h: &Harness| {
        let campaign = h.flow().and_then(|f| f.campaign());
        let roster = &campaign.unwrap_or_else(|| panic!("no campaign")).roster;
        roster[3].clone()
    };
    assert_eq!(scout(&h).name, "Test Scout");
    assert_eq!(scout(&h).loadout.weapon_count(), 0);
    // Restart Battle: Preparations again, the trade as it was left.
    h.keys("f f f d Down Down f f");
    assert_eq!(h.screens(), ["title", "preparations"]);
    let prep = h.flow().and_then(|f| f.preparations());
    let prep = prep.unwrap_or_else(|| panic!("no preparations")).prep();
    assert_eq!(prep.bench[0].loadout.weapon_count(), 0);
    assert_eq!(prep.setup.units[2].loadout.weapon_count(), 2);
}
