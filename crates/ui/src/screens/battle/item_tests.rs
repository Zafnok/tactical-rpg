//! Tests of the `Item` and `Equip` menus (ticket 0407): the pack list, item
//! targeting and the heal it does, the heal popup, the equip list, and that
//! cancelling at each level leaves the battle unchanged.

use insta::assert_snapshot;
use trpg_core::{BattlePack, BattleState, Equipped, ItemId, Objective, Pos, StatValue, UnitId};

use super::items::{ItemTargeting, item_targets, pack_groups};
use super::mode::{MenuEntry, Mode};
use super::testing::{battle_packed, quick_units};
use super::{BattleScreen, HealPopup, TIMINGS};
use crate::FrameInput;
use crate::color::UiColor;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::tests::ctx;
use crate::screen::{Ctx, Screen};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// The Quick Battle with the lord (unit 1, at (3, 5)) and the knight (unit
/// 2, moved to (4, 5), beside it) `lord_hurt` and `knight_hurt` HP below
/// full, and a pack of `pack` (three potions and an elixir, or as given).
fn hurt_pair(c: &Ctx, lord_hurt: StatValue, knight_hurt: StatValue, pack: &[&str]) -> BattleState {
    let (map, mut units) = quick_units(c);
    units[1].pos = p(4, 5);
    units[0].hp = units[0].stats.hp - lord_hurt;
    units[1].hp = units[1].stats.hp - knight_hurt;
    let items = pack.iter().map(|id| ItemId::new(id)).collect();
    let pack = BattlePack::bring(items, 6).expect("fits");
    let rout = Objective::Rout { turn_limit: None };
    battle_packed(c, map, units, rout, 0, pack)
}

fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
}

fn wait(s: &mut BattleScreen, c: &mut Ctx, seconds: f32) {
    s.update(c, &FrameInput::new(vec![], seconds, vec![]));
}

fn render(s: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let stale = Cell::new(
        'x',
        crate::color::Rgb::new(1, 2, 3),
        crate::color::Rgb::new(1, 2, 3),
    );
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(c, &mut buf);
    buf
}

/// The lord's action menu, opened without moving.
fn lord_menu(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    step(&mut s, c, &[Action::Confirm, Action::Confirm]);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    s
}

/// Focus is on `Wait`; `ups` steps up to the entry wanted.
const ITEM: [Action; 3] = [Action::CursorUp, Action::CursorUp, Action::Confirm];
const EQUIP: [Action; 2] = [Action::CursorUp, Action::Confirm];

fn text(buf: &GlyphBuffer, x: i32, y: i32, w: i32) -> String {
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim_end()
        .to_owned()
}

#[test]
fn item_is_enabled_only_with_a_usable_item_and_someone_hurt() {
    let mut c = ctx();
    let enabled = |s: &BattleScreen| {
        let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        let i = entries.iter().position(|&e| e == MenuEntry::Item).unwrap();
        menu.items()[i].enabled
    };
    // Someone hurt beside the lord, potions in the pack.
    let s = {
        let st = hurt_pair(&c, 0, 4, &["potion"]);
        lord_menu(&mut c, st)
    };
    assert!(enabled(&s));
    // Nobody hurt: nothing to heal.
    let s = {
        let st = hurt_pair(&c, 0, 0, &["potion"]);
        lord_menu(&mut c, st)
    };
    assert!(!enabled(&s));
    // Hurt, but an empty pack.
    let s = {
        let st = hurt_pair(&c, 3, 4, &[]);
        lord_menu(&mut c, st)
    };
    assert!(!enabled(&s));
    // The lord itself hurt is enough.
    let s = {
        let st = hurt_pair(&c, 3, 0, &["potion"]);
        lord_menu(&mut c, st)
    };
    assert!(enabled(&s));
}

#[test]
fn the_pack_groups_by_item_and_targets_are_self_then_adjacent_allies_in_order() {
    let c = ctx();
    let state = hurt_pair(&c, 3, 4, &["potion", "elixir", "potion", "potion"]);
    let groups = pack_groups(&state, UnitId(1), p(3, 5));
    let shown: Vec<_> = groups
        .iter()
        .map(|g| (g.item.0.as_str(), g.index, g.count))
        .collect();
    assert_eq!(shown, [("potion", 0, 3), ("elixir", 1, 1)]);
    // The lord at (3, 5) and the knight at (4, 5): one row, left first.
    assert_eq!(
        item_targets(&state, UnitId(1), p(3, 5)),
        [UnitId(1), UnitId(2)]
    );
    // Moved a tile: the knight is no longer beside it.
    assert_eq!(item_targets(&state, UnitId(1), p(3, 4)), [UnitId(1)]);
    // Enemies never.
    assert!(item_targets(&state, UnitId(4), p(8, 2)).len() <= 1);
}

#[test]
fn select_move_item_potion_ally_heals_the_ally_and_ends_the_action() {
    let mut c = ctx();
    let state = hurt_pair(&c, 0, 15, &["potion", "potion", "potion"]);
    let max = state.unit(UnitId(2)).unwrap().stats.hp;
    let mut s = lord_menu(&mut c, state);
    step(&mut s, &mut c, &ITEM);
    assert!(matches!(s.mode(), Mode::ItemMenu { .. }), "{:?}", s.mode());
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::ItemTarget(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    // Only the knight is hurt: it is the one target, under the cursor.
    assert_eq!(t.target(), UnitId(2));
    assert_eq!(s.cursor().pos, p(4, 5));
    step(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.mode(), &Mode::default());
    let state = s.state();
    assert_eq!(state.unit(UnitId(2)).unwrap().hp, max - 5);
    assert!(state.unit(UnitId(1)).unwrap().acted);
    assert!(!state.unit(UnitId(2)).unwrap().acted);
    assert_eq!(state.pack().items.len(), 2);
    // The popup floats over the knight, then goes.
    assert_eq!(
        s.popups(),
        [HealPopup {
            pos: p(4, 5),
            amount: 10,
            t: 0.0
        }]
    );
    wait(&mut s, &mut c, TIMINGS.heal_popup + 0.1);
    assert!(s.popups().is_empty());
}

#[test]
fn an_item_can_be_used_on_oneself_and_targets_cycle_both_ways() {
    let mut c = ctx();
    let mut s = {
        let st = hurt_pair(&c, 4, 6, &["potion"]);
        lord_menu(&mut c, st)
    };
    step(&mut s, &mut c, &ITEM);
    step(&mut s, &mut c, &[Action::Confirm]);
    let target = |s: &BattleScreen| match s.mode() {
        Mode::ItemTarget(t) => t.target(),
        m => panic!("{m:?}"),
    };
    // Starts on the user; both cycle keys wrap.
    assert_eq!(target(&s), UnitId(1));
    step(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!((target(&s), s.cursor().pos), (UnitId(2), p(4, 5)));
    step(&mut s, &mut c, &[Action::NextUnit]);
    assert_eq!(target(&s), UnitId(1));
    step(&mut s, &mut c, &[Action::PrevUnit]);
    assert_eq!(target(&s), UnitId(2));
    step(&mut s, &mut c, &[Action::CursorLeft]);
    assert_eq!((target(&s), s.cursor().pos), (UnitId(1), p(3, 5)));
    let max = s.state().unit(UnitId(1)).unwrap().stats.hp;
    step(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.state().unit(UnitId(1)).unwrap().hp, max);
    assert!(s.state().pack().items.is_empty());
}

#[test]
fn the_preview_line_shows_hp_before_and_after_capped_at_max() {
    let c = ctx();
    let state = hurt_pair(&c, 4, 15, &["potion", "elixir"]);
    let (lord, knight) = (
        state.unit(UnitId(1)).unwrap(),
        state.unit(UnitId(2)).unwrap(),
    );
    let (sel, groups) = {
        let sel = super::mode::Selection::new(&state, UnitId(1)).unwrap();
        let groups = pack_groups(&state, UnitId(1), p(3, 5));
        (sel, groups)
    };
    let menu = super::items::pack_menu(&state, &groups);
    let mut t = ItemTargeting::new(sel.clone(), menu.clone(), groups.clone(), 0).unwrap();
    // The potion on the lord, 4 down: only 4 restored.
    let line = t.preview(&state);
    assert_eq!(
        line,
        format!(
            "Potion on {}: HP {} → {}",
            lord.name, lord.hp, lord.stats.hp
        )
    );
    t.cycle(true);
    assert_eq!(
        t.preview(&state),
        format!(
            "Potion on {}: HP {} → {}",
            knight.name,
            knight.hp,
            knight.hp + 10
        )
    );
    // The elixir restores all.
    let elixir = ItemTargeting::new(sel, menu, groups, 1).unwrap();
    assert!(elixir.preview(&state).starts_with("Elixir on "));
}

#[test]
fn cancelling_at_each_level_leaves_the_battle_unchanged() {
    let mut c = ctx();
    let state = hurt_pair(&c, 3, 4, &["potion", "potion"]);
    let before = state.clone();
    let mut s = lord_menu(&mut c, state);
    step(&mut s, &mut c, &ITEM);
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::ItemTarget(_)));
    // Target → pack list (cursor back on the unit) → action menu, on Item.
    step(&mut s, &mut c, &[Action::CursorRight, Action::Cancel]);
    assert!(matches!(s.mode(), Mode::ItemMenu { .. }), "{:?}", s.mode());
    assert_eq!(s.cursor().pos, p(3, 5));
    step(&mut s, &mut c, &[Action::Cancel]);
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Item);
    assert_eq!(s.state(), &before);
    // Equip list → action menu, on Equip.
    step(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    assert!(matches!(s.mode(), Mode::EquipMenu { .. }), "{:?}", s.mode());
    step(&mut s, &mut c, &[Action::Cancel]);
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Equip);
    assert_eq!(s.state(), &before);
    assert!(s.popups().is_empty());
}

#[test]
fn equip_swaps_the_weapon_and_the_unit_can_still_act() {
    let mut c = ctx();
    let state = hurt_pair(&c, 0, 0, &["potion"]);
    let lord = state.unit(UnitId(1)).unwrap();
    assert_eq!(lord.loadout.equipped, Some(Equipped::Weapon(0)));
    let mut s = lord_menu(&mut c, state);
    step(&mut s, &mut c, &EQUIP);
    let Mode::EquipMenu { menu, choices, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    // The equipped weapon is focused; the next one is chosen.
    assert_eq!((menu.focus(), choices.len()), (0, 2));
    step(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    let lord = s.state().unit(UnitId(1)).unwrap();
    assert_eq!(lord.loadout.equipped, Some(Equipped::Weapon(1)));
    assert!(!lord.acted, "equipping is free");
    // Back at the action menu, at the same tile, on Equip.
    let Mode::ActionMenu {
        menu, entries, sel, ..
    } = s.mode()
    else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Equip);
    assert_eq!(sel.dest(), p(3, 5));
    // It can still act: Wait.
    step(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    assert!(s.state().unit(UnitId(1)).unwrap().acted);
    assert_eq!(s.mode(), &Mode::default());
}

#[test]
fn equip_needs_two_usable_weapons() {
    let mut c = ctx();
    let (map, mut units) = quick_units(&c);
    units[0].loadout.weapons[1] = None;
    let rout = Objective::Rout { turn_limit: None };
    let state = battle_packed(&c, map, units, rout, 0, BattlePack::default());
    let s = lord_menu(&mut c, state);
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let i = entries.iter().position(|&e| e == MenuEntry::Equip).unwrap();
    assert!(!menu.items()[i].enabled);
}

#[test]
fn pack_menu_snapshot() {
    let mut c = ctx();
    let pack = ["potion", "elixir", "potion", "potion"];
    let mut s = {
        let st = hurt_pair(&c, 3, 12, &pack);
        lord_menu(&mut c, st)
    };
    step(&mut s, &mut c, &ITEM);
    let buf = render(&s, &c);
    assert!(text(&buf, 30, 15, 40).contains("Pack 4/6"));
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn item_target_snapshot() {
    let mut c = ctx();
    let pack = ["potion", "elixir", "potion", "potion"];
    let mut s = {
        let st = hurt_pair(&c, 3, 12, &pack);
        lord_menu(&mut c, st)
    };
    step(&mut s, &mut c, &ITEM);
    step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
    let buf = render(&s, &c);
    let message = text(&buf, 1, 30, 60);
    assert!(message.contains(": HP "), "{message}");
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn equip_menu_with_a_broken_weapon_snapshot() {
    let mut c = ctx();
    let (map, mut units) = quick_units(&c);
    if let Some(w) = units[0].loadout.weapons[1].as_mut() {
        w.durability_left = 0;
    }
    if let Some(w) = units[0].loadout.weapons[0].as_mut() {
        w.durability_left = 12;
    }
    let rout = Objective::Rout { turn_limit: None };
    let state = battle_packed(&c, map, units, rout, 0, BattlePack::default());
    let mut s = lord_menu(&mut c, state);
    step(&mut s, &mut c, &EQUIP);
    let buf = render(&s, &c);
    // `(broken)` in the warning colour, after the durability `0/25`.
    let row = (0..i32::from(CONSOLE_H))
        .find(|&y| text(&buf, 0, y, i32::from(CONSOLE_W)).contains("(broken)"))
        .expect("a broken weapon is tagged");
    let cell = (0..i32::from(CONSOLE_W))
        .find(|&cx| text(&buf, cx, row, 8) == "(broken)")
        .expect("cell");
    assert_eq!(
        buf.get(cell, row).unwrap().fg,
        c.palette.get(UiColor::HpLow)
    );
    assert!(text(&buf, 0, row, i32::from(CONSOLE_W)).contains("0/25"));
    assert_snapshot!(buf.to_snapshot(&c.palette));
}
