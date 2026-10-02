//! Tests of the Preparations screen on the debug Quick Battle
//! (`assets/battles/quick.ron`): the test lord (swords), knight (spears,
//! medium and heavy armour) and archer, the test scout left out of the
//! battle with a Steel Bow, a stock of three weapons, three armours, three
//! accessories, six Potions and two Elixirs, and a pack cap of 6.

use insta::assert_snapshot;
use trpg_content::battle_campaign;
use trpg_core::{GameMode, WeaponRank};

use super::*;
use crate::audio::AudioRequest;
use crate::harness::Harness;
use crate::input::Layout;
use crate::screen::tests::ctx;
use crate::screens::battle::QUICK_BATTLE;

use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};

/// The Quick Battle's setup and bench, before Preparations.
fn setup(c: &Ctx) -> Preparations {
    let def = &c.content.battles[QUICK_BATTLE];
    let campaign = battle_campaign(&c.content, def, GameMode::Classic, c.lead.clone());
    Preparations {
        setup: campaign.battle_setup(def, &c.content.tables()),
        bench: campaign.bench(def),
    }
}

fn screen(c: &Ctx) -> PreparationsScreen {
    PreparationsScreen::new(setup(c), true)
}

/// One update with `actions`: the transition, as text.
fn update(s: &mut PreparationsScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    format!("{:?}", s.update(c, &input))
}

/// The sounds `actions` play.
fn sounds(s: &mut PreparationsScreen, c: &mut Ctx, actions: &[Action]) -> Vec<String> {
    c.audio.take();
    update(s, c, actions);
    c.audio
        .take()
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlaySound { .. }))
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

fn potion() -> ItemId {
    ItemId::new("potion")
}

/// The lord's weapons, by slot.
fn lord_weapons(s: &PreparationsScreen) -> Vec<Option<String>> {
    let lord = &s.setup().units[0];
    (0..3)
        .map(|i| lord.loadout.weapon(i).map(|w| w.def.0.clone()))
        .collect()
}

/// The texts of the stock rows.
fn stock_texts(s: &PreparationsScreen) -> Vec<String> {
    s.stock_rows().into_iter().map(|r| r.text).collect()
}

/// The Quick Battle's Preparations, through the title.
fn harness() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "preparations"]);
    h
}

#[test]
fn opens_on_the_loadouts_tab_with_an_empty_pack() {
    let c = ctx();
    let s = screen(&c);
    assert_eq!(s.name(), "preparations");
    assert!(!s.is_overlay());
    assert_eq!((s.tab(), s.focus()), (Tab::Loadouts, Focus::Tabs));
    assert_eq!(s.outcome(), None);
    assert_eq!(s.message(), None);
    assert!(!s.is_asking_leave());
    assert_eq!(s.pack_header(), "Pack 0/6");
    assert!(s.packed().is_empty());
    assert_eq!(
        s.spare(),
        [(ItemId::new("elixir"), 2), (ItemId::new("potion"), 6)]
    );
    assert_eq!(Tab::ALL.map(Tab::label), ["Loadouts", "Pack", "Fight!"]);
}

#[test]
fn left_and_right_pick_a_tab_and_fight_starts_the_battle() {
    let mut c = ctx();
    let mut s = screen(&c);
    assert_eq!(sounds(&mut s, &mut c, &[CursorRight]), ["menu_move"]);
    assert_eq!(s.tab(), Tab::Pack);
    update(&mut s, &mut c, &[CursorRight]);
    assert_eq!(s.tab(), Tab::Fight);
    // Down opens a tab, but never starts the battle.
    assert_eq!(update(&mut s, &mut c, &[CursorDown, CursorUp]), "None");
    assert_eq!(s.focus(), Focus::Tabs);
    update(&mut s, &mut c, &[CursorRight]);
    assert_eq!(s.tab(), Tab::Loadouts, "wraps");
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(s.tab(), Tab::Fight);
    assert_eq!(update(&mut s, &mut c, &[Confirm]), "Pop");
    assert_eq!(s.outcome(), Some(PrepOutcome::Fight));
}

#[test]
fn a_stock_weapon_goes_into_an_empty_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    // Loadouts → the lord → his third slot → the stock.
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_select"]);
    assert_eq!(s.focus(), Focus::Units);
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.focus(), Focus::Slots);
    assert_eq!(sounds(&mut s, &mut c, &[CursorDown]), ["menu_move"]);
    update(&mut s, &mut c, &[CursorDown, Confirm]);
    assert_eq!(s.focus(), Focus::Stock);
    assert_eq!(
        stock_texts(&s),
        [
            "Steel Spear     can't use spears",
            "Iron Axe        can't use axes",
            "Iron Sword      Mt 5 Hit 90 Wt 2 Rng1 20/20",
        ]
    );
    assert_eq!(
        s.speed_line().as_deref(),
        Some("AS 7 with no weapon"),
        "a row he can't take changes nothing"
    );
    update(&mut s, &mut c, &[CursorUp]);
    assert_eq!(s.speed_line().as_deref(), Some("AS 7 → 6 with Iron Sword"));
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_select"]);
    assert_eq!(s.focus(), Focus::Slots);
    assert_eq!(
        lord_weapons(&s),
        ["iron_sword", "steel_sword", "iron_sword"].map(|w| Some(w.to_owned()))
    );
    assert_eq!(s.setup().stock.weapons.len(), 2);
    assert_eq!(s.speed_line().as_deref(), Some("AS 6 with Iron Sword"));
    // The slot now offers to put it back.
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(stock_texts(&s)[0], PUT_BACK);
    assert_eq!(s.speed_line().as_deref(), Some("AS 6 → 7 with no weapon"));
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(lord_weapons(&s)[2], None);
    assert_eq!(s.setup().stock.weapons.len(), 3);
}

#[test]
fn an_item_the_unit_cant_use_is_refused_with_the_reason() {
    let mut c = ctx();
    let mut s = screen(&c);
    update(&mut s, &mut c, &[Confirm, Confirm, CursorDown, CursorDown]);
    update(&mut s, &mut c, &[Confirm]);
    let before = s.setup().clone();
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.message(), Some("Test Lord can't use spears."));
    assert_eq!(s.focus(), Focus::Stock);
    assert_eq!(s.setup(), &before);
    assert!(s.stock_rows()[0].unusable.is_some());
    // The next key clears the message.
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.message(), None);
}

#[test]
fn reasons_read_as_the_player_sees_them() {
    assert_eq!(reason_text(Unusable::Rank(WeaponRank::D)), "needs rank D");
    let kinds = [
        (WeaponKind::Sword, "can't use swords"),
        (WeaponKind::Spear, "can't use spears"),
        (WeaponKind::Axe, "can't use axes"),
        (WeaponKind::Bow, "can't use bows"),
        (WeaponKind::Gauntlet, "can't use gauntlets"),
    ];
    for (kind, text) in kinds {
        assert_eq!(reason_text(Unusable::Kind(kind)), text);
    }
    let weights = [
        (ArmourWeight::Light, "can't wear light armour"),
        (ArmourWeight::Medium, "can't wear medium armour"),
        (ArmourWeight::Heavy, "can't wear heavy armour"),
    ];
    for (weight, text) in weights {
        assert_eq!(reason_text(Unusable::Armour(weight)), text);
    }
}

#[test]
fn armour_and_accessories_come_from_the_stock_with_their_counts() {
    let mut c = ctx();
    let mut s = screen(&c);
    // The lord's armour slot: he wears a Leather Vest.
    update(
        &mut s,
        &mut c,
        &[Confirm, Confirm, CursorUp, CursorUp, Confirm],
    );
    assert_eq!(
        stock_texts(&s),
        [
            PUT_BACK,
            "Chain Mail      ×1  Def +3  Wt 2",
            "Iron Plate      can't wear heavy armour",
            "Warded Robe     ×1  Res +2  Wt 0",
        ]
    );
    // With no weapon slot highlighted, the speed is the equipped weapon's.
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.speed_line().as_deref(), Some("AS 6 → 4 with Iron Sword"));
    update(&mut s, &mut c, &[Confirm]);
    let lord = &s.setup().units[0];
    assert_eq!(lord.loadout.armour, Some(ItemId::new("chain_mail")));
    assert_eq!(s.setup().stock.count(&ItemId::new("leather_vest")), 1);
    assert_eq!(s.speed_line().as_deref(), Some("AS 4 with Iron Sword"));
    // The accessory slot: empty, so nothing to put back.
    update(&mut s, &mut c, &[CursorDown, Confirm]);
    assert_eq!(
        stock_texts(&s),
        [
            "Focus Charm     ×1  Dex +2",
            "Power Ring      ×1  Str +2",
            "Speed Ring      ×1  Spd +2",
        ]
    );
    update(&mut s, &mut c, &[CursorUp]);
    assert_eq!(s.speed_line().as_deref(), Some("AS 4 → 6 with Iron Sword"));
    update(&mut s, &mut c, &[Confirm]);
    let lord = &s.setup().units[0];
    assert_eq!(lord.loadout.accessory, Some(ItemId::new("speed_ring")));
}

#[test]
fn a_slot_with_nothing_in_stock_says_so() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.prep.setup.stock = trpg_core::Stock::default();
    // The lord's empty third slot.
    update(&mut s, &mut c, &[Confirm, Confirm, CursorDown, CursorDown]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.focus(), Focus::Slots);
    assert_eq!(s.message(), Some(NOTHING_IN_STOCK));
    // A filled slot can still be put back.
    update(&mut s, &mut c, &[CursorUp, Confirm]);
    assert_eq!(s.focus(), Focus::Stock);
    assert_eq!(stock_texts(&s), [PUT_BACK]);
}

#[test]
fn cancel_goes_back_one_list_at_a_time() {
    let mut c = ctx();
    let mut s = screen(&c);
    update(&mut s, &mut c, &[Confirm, CursorDown, Confirm, Confirm]);
    assert_eq!(s.focus(), Focus::Stock);
    // The knight: the second unit.
    assert_eq!(s.unit().map(|u| u.name.as_str()), Some("Test Knight"));
    for back in [Focus::Slots, Focus::Units, Focus::Tabs] {
        assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
        assert_eq!(s.focus(), back);
    }
    // Left and Right do nothing inside the loadouts.
    update(&mut s, &mut c, &[Confirm]);
    assert!(sounds(&mut s, &mut c, &[CursorLeft, CursorRight]).is_empty());
    assert_eq!((s.tab(), s.focus()), (Tab::Loadouts, Focus::Units));
    // The unit list wraps, and a new unit starts on its first slot.
    update(
        &mut s,
        &mut c,
        &[Confirm, CursorDown, Cancel, CursorUp, CursorUp],
    );
    assert_eq!(s.unit().map(|u| u.name.as_str()), Some("Test Scout"));
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.gear_slot(), Some(GearSlot::Weapon(0)));
}

#[test]
fn the_pack_fills_from_the_stock_up_to_the_cap() {
    let mut c = ctx();
    let mut s = screen(&c);
    update(&mut s, &mut c, &[CursorRight, Confirm]);
    assert_eq!((s.tab(), s.focus()), (Tab::Pack, Focus::Spare));
    // Elixir, then Potion: two Elixirs and four Potions fill the pack.
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_select"]);
    update(&mut s, &mut c, &[Confirm]);
    // The Elixirs are gone from the stock: the cursor is on the Potions.
    assert_eq!(s.spare(), [(potion(), 6)]);
    update(&mut s, &mut c, &[Confirm, Confirm, Confirm, Confirm]);
    assert_eq!(s.pack_header(), "Pack 6/6");
    assert_eq!(s.packed(), [(ItemId::new("elixir"), 2), (potion(), 4)]);
    assert_eq!(s.message(), None);
    let before = s.setup().clone();
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.message(), Some("The pack is full (6/6)."));
    assert_eq!(s.setup(), &before);
    assert_eq!(s.setup().stock.count(&potion()), 2);
}

#[test]
fn pack_items_go_back_to_the_stock() {
    let mut c = ctx();
    let mut s = screen(&c);
    // Nothing packed: Right stays on the stock.
    update(&mut s, &mut c, &[CursorRight, Confirm]);
    assert!(sounds(&mut s, &mut c, &[CursorRight]).is_empty());
    assert_eq!(s.focus(), Focus::Spare);
    update(&mut s, &mut c, &[CursorDown, Confirm, Confirm]);
    assert_eq!(s.packed(), [(potion(), 2)]);
    assert_eq!(sounds(&mut s, &mut c, &[CursorRight]), ["menu_move"]);
    assert_eq!(s.focus(), Focus::Pack);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_select"]);
    assert_eq!(s.packed(), [(potion(), 1)]);
    assert_eq!(s.focus(), Focus::Pack);
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(s.focus(), Focus::Spare);
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(s.focus(), Focus::Pack);
    // The last one out: back to the stock's list.
    update(&mut s, &mut c, &[Confirm]);
    assert!(s.packed().is_empty());
    assert_eq!(s.focus(), Focus::Spare);
    assert_eq!(s.setup().stock.count(&potion()), 6);
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
    assert_eq!(s.focus(), Focus::Tabs);
}

#[test]
fn the_pack_tab_opens_on_the_pack_when_the_stock_has_no_consumables() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.prep.setup.stock.items.retain(|id, _| *id != potion());
    update(&mut s, &mut c, &[CursorRight, Confirm, Confirm, Confirm]);
    // Both Elixirs packed, none left: the cursor moved to the pack.
    assert!(s.spare().is_empty());
    assert_eq!(s.focus(), Focus::Pack);
    update(&mut s, &mut c, &[Cancel, Confirm]);
    assert_eq!(s.focus(), Focus::Pack);
    // With neither, the tab can't be opened.
    s.prep.setup.pack.items.clear();
    update(&mut s, &mut c, &[Cancel]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.focus(), Focus::Tabs);
}

#[test]
fn leaving_asks_first_and_only_when_allowed() {
    let mut c = ctx();
    let mut s = screen(&c);
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_select"]);
    assert!(s.is_asking_leave());
    // Other keys do nothing while it asks.
    assert_eq!(update(&mut s, &mut c, &[CursorRight, CursorDown]), "None");
    assert_eq!((s.tab(), s.focus()), (Tab::Loadouts, Focus::Tabs));
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
    assert!(!s.is_asking_leave());
    assert_eq!(update(&mut s, &mut c, &[Cancel, Confirm]), "Pop");
    assert_eq!(s.outcome(), Some(PrepOutcome::Leave));

    let mut s = PreparationsScreen::new(setup(&c), false);
    assert_eq!(s.prep().bench.len(), 1);
    assert!(sounds(&mut s, &mut c, &[Cancel]).is_empty());
    assert!(!s.is_asking_leave());
    assert!(!s.help(&c).contains("leave"));
}

#[test]
fn help_names_the_players_keys() {
    let mut c = ctx();
    let mut s = screen(&c);
    assert_eq!(s.help(&c), "arrows choose · f open · d leave");
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.help(&c), "arrows choose · f change gear · d back");
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.help(&c), "arrows choose · f stock · d back");
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.help(&c), "arrows choose · f take · d back");
    update(
        &mut s,
        &mut c,
        &[Cancel, Cancel, Cancel, CursorRight, Confirm],
    );
    assert_eq!(s.help(&c), "arrows choose · f pack it · d back");
    update(&mut s, &mut c, &[Confirm, CursorRight]);
    assert_eq!(s.help(&c), "arrows choose · f put back · d back");
    update(&mut s, &mut c, &[Cancel, CursorRight]);
    assert_eq!(s.help(&c), "arrows choose · f start the battle · d leave");
    update(&mut s, &mut c, &[Cancel]);
    assert_eq!(s.help(&c), "f yes / d no");
    // The left-handed layout has other keys.
    c.use_layout(Layout::LeftHanded);
    assert_eq!(s.help(&c), "j yes / k no");
}

#[test]
fn lists_wrap_and_scroll() {
    assert_eq!(step(0, 3, true), 1);
    assert_eq!(step(2, 3, true), 0);
    assert_eq!(step(0, 3, false), 2);
    assert_eq!(step(1, 3, false), 0);
    assert_eq!(step(0, 0, true), 0);
    assert_eq!(step(0, 1, false), 0);
    // A window of 3 rows over 5: it moves only once the cursor leaves it.
    let starts: Vec<usize> = (0..5).map(|f| window_start(f, 5, 3)).collect();
    assert_eq!(starts, [0, 0, 0, 1, 2]);
    assert_eq!(window_start(1, 2, 3), 0);
    assert_eq!(window_start(0, 0, 3), 0);
}

#[test]
fn loadouts_tab_snapshot() {
    let mut h = harness();
    // The lord's third weapon slot, on the Iron Sword he can take.
    h.keys("f f Down Down f Up");
    assert_snapshot!(h.snapshot());
}

#[test]
fn pack_tab_at_the_cap_snapshot() {
    let mut h = harness();
    h.keys("Right f f Down f f f f f f");
    assert_snapshot!(h.snapshot());
}

#[test]
fn fight_tab_and_leave_question_snapshot() {
    let mut h = harness();
    h.keys("Left d");
    assert_snapshot!(h.snapshot());
}

/// Nick (0408): with two archers and only one in the battle, the better
/// bow of the one left out can go to the one who fights.
#[test]
fn a_benched_units_gear_is_traded_through_the_stock() {
    let mut c = ctx();
    let mut s = screen(&c);
    // The scout is listed last, after the deployed units.
    update(&mut s, &mut c, &[Confirm, CursorUp, Confirm]);
    assert_eq!(s.who(), Some(PrepUnit::Benched(0)));
    assert_eq!(s.unit().map(|u| u.name.as_str()), Some("Test Scout"));
    assert_eq!(s.speed_line().as_deref(), Some("AS 2 with Steel Bow"));
    // Her Steel Bow goes back to the stock.
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(stock_texts(&s)[0], PUT_BACK);
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.prep().bench[0].loadout.weapon(0), None);
    assert_eq!(s.prep().bench[0].loadout.equipped, None);
    // The archer (the unit above her) takes it in his second slot.
    update(
        &mut s,
        &mut c,
        &[Cancel, CursorUp, Confirm, CursorDown, Confirm],
    );
    assert_eq!(s.unit().map(|u| u.name.as_str()), Some("Test Archer"));
    assert_eq!(
        stock_texts(&s).last().map(String::as_str),
        Some("Steel Bow       Mt 9 Hit 70 Wt 5 Rng2 25/25")
    );
    update(&mut s, &mut c, &[CursorUp, Confirm]);
    let archer = &s.setup().units[2];
    assert_eq!(
        archer.loadout.weapon(1).map(|w| w.def.0.as_str()),
        Some("steel_bow")
    );
    // The scout can take a stock item too, by the same rules.
    update(&mut s, &mut c, &[Cancel, CursorDown, Confirm, Confirm]);
    assert_eq!(s.who(), Some(PrepUnit::Benched(0)));
    assert!(stock_texts(&s)[0].ends_with("can't use spears"));
}

#[test]
fn a_benched_unit_snapshot() {
    let mut h = harness();
    h.keys("f Up f");
    assert_snapshot!(h.snapshot());
}
