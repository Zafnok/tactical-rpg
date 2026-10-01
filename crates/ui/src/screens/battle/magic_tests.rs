//! Tests of the `Magic` menu (ticket 0410): the spell list, casting on an
//! enemy (the forecast with its affinity markers), on an ally (the heal
//! preview and popup) and on a tile (the terrain preview, the changed tile
//! and its flash), spells in the `Equip` menu and on the info screen, and
//! that cancelling at each level leaves the battle unchanged.

use insta::assert_snapshot;
use trpg_core::{
    Affinity, BattleState, CastTarget, ClassId, Command, Element, Equipped, Objective, Phase, Pos,
    SkillId, SpellId, StatValue, UnitAction, UnitId,
};

use super::forecast::{AFFINITY_ROW, LEFT_X, NAME_ROW, STRIKE_ROW};
use super::items::{can_equip, equip_choices, equip_menu};
use super::magic::{
    CastTargeting, can_cast, cast, cast_targets, knows_spells, reaches_an_enemy, spell_choices,
    spell_label, spell_menu, target_pos,
};
use super::mode::{Effect, MenuEntry, Mode, Selection, menu_entries, open_menu};
use super::sounds::step_sound;
use super::testing::{battle_with, quick_units, through_ai_phases};
use super::{BattleScreen, HealPopup, OVERLAY_BLEND, TERRAIN_FLASH_S, TerrainFlash, quick_battle};
use crate::FrameInput;
use crate::audio::MenuSound;
use crate::color::{Rgb, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::harness::Harness;
use crate::input::Action;
use crate::screen::tests::ctx;
use crate::screen::{Ctx, Screen};

/// The Quick Battle's mage.
const MAGE: UnitId = UnitId(8);

/// Its frost elemental: weak to Fire, absorbs Ice.
const ELEMENTAL: UnitId = UnitId(7);

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn sid(id: &str) -> SpellId {
    SpellId::new(id)
}

fn on_unit(id: u32) -> CastTarget {
    CastTarget::Unit(UnitId(id))
}

fn on_tile(x: i32, y: i32) -> CastTarget {
    CastTarget::Tile(p(x, y))
}

/// The Quick Battle with the mage (unit 8) at (0, 4), in reach of two
/// forests ((1, 5) and (0, 6)), a water tile (1, 3) and a sea tile (0, 2);
/// the frost elemental (unit 7) beside it at (1, 4) and a brigand (unit 4)
/// at (2, 4); the knight (unit 2) below it at (0, 5), `knight_hurt` HP
/// down; the archer out of the way at (4, 6).
fn field(c: &Ctx, knight_hurt: StatValue) -> BattleState {
    let (map, mut units) = quick_units(c);
    units[7].pos = p(0, 4);
    units[6].pos = p(1, 4);
    units[3].pos = p(2, 4);
    units[1].pos = p(0, 5);
    units[1].hp -= knight_hurt;
    units[2].pos = p(4, 6);
    battle_with(c, map, units, Objective::Rout { turn_limit: None })
}

fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
}

fn wait(s: &mut BattleScreen, c: &mut Ctx, seconds: f32) {
    s.update(c, &FrameInput::new(vec![], seconds, vec![]));
}

fn render(s: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(c, &mut buf);
    buf
}

/// Cells `x..x + w` of row `y`, trimmed on the right.
fn text(buf: &GlyphBuffer, x: i32, y: i32, w: i32) -> String {
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// Whether some row of `buf` shows `s`.
fn shows(buf: &GlyphBuffer, s: &str) -> bool {
    (0..i32::from(CONSOLE_H)).any(|y| text(buf, 0, y, i32::from(CONSOLE_W)).contains(s))
}

/// The left cell of tile `(x, y)` (`test_small` is centred: the tile starts
/// at cell `(2 × (x + 10), y + 11)`).
fn cell(x: i32, y: i32) -> (i32, i32) {
    (2 * (x + 10), y + 11)
}

/// The glyphs and colours of the tile at `(x, y)`.
fn tile(buf: &GlyphBuffer, x: i32, y: i32) -> (String, Rgb, Rgb) {
    let (cx, cy) = cell(x, y);
    let at = buf.get(cx, cy).unwrap();
    (text(buf, cx, cy, 2), at.fg, at.bg)
}

/// The mage of `state` selected where it stands, its action menu open.
fn mage_menu(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let at = state.unit(MAGE).unwrap().pos;
    let mut s = BattleScreen::new(state);
    s.cursor.jump(at);
    step(&mut s, c, &[Action::Confirm, Action::Confirm]);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    s
}

/// Chooses `entry` in the open action menu.
fn choose(s: &mut BattleScreen, c: &mut Ctx, entry: MenuEntry) {
    for _ in 0..8 {
        let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        if entries.get(menu.focus()) == Some(&entry) {
            step(s, c, &[Action::Confirm]);
            return;
        }
        step(s, c, &[Action::CursorUp]);
    }
    panic!("no {entry:?} to choose");
}

/// The mage's spell list of `state`, open.
fn spell_list(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = mage_menu(c, state);
    choose(&mut s, c, MenuEntry::Magic);
    assert!(matches!(s.mode(), Mode::SpellMenu { .. }), "{:?}", s.mode());
    s
}

/// Picking the target of line `line` of the mage's spell list of `state`
/// (0 Fire, 1 Frost, 2 Heal).
fn casting(c: &mut Ctx, state: BattleState, line: usize) -> BattleScreen {
    let mut s = spell_list(c, state);
    // The list opens on Fire, the equipped spell.
    step(&mut s, c, &vec![Action::CursorDown; line]);
    step(&mut s, c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::CastTarget(_)), "{:?}", s.mode());
    s
}

fn targeting(s: &BattleScreen) -> &CastTargeting {
    match s.mode() {
        Mode::CastTarget(t) => t,
        m => panic!("{m:?}"),
    }
}

fn uses(s: &BattleState, spell: &str) -> u8 {
    s.unit(MAGE).unwrap().spells.uses_left(&sid(spell))
}

fn terrain(c: &Ctx, s: &BattleState, x: i32, y: i32) -> String {
    let id = *s.map().tiles.get(p(x, y)).unwrap();
    c.content.terrain.display.get(id).unwrap().id.clone()
}

// ---- The spell list --------------------------------------------------------------

#[test]
fn a_spells_targets_are_what_the_core_accepts_in_reading_order() {
    let c = ctx();
    let s = field(&c, 18);
    let spell = |id: &str| c.content.spells.get(&sid(id)).unwrap();
    let from = |dest, id| cast_targets(&s, MAGE, dest, spell(id));
    // Fire: the enemies in range (never the knight), then the forests.
    assert_eq!(
        from(p(0, 4), "fire"),
        [on_unit(7), on_unit(4), on_tile(1, 5), on_tile(0, 6)]
    );
    // Frost: the sea and the water above, then the enemies.
    assert_eq!(
        from(p(0, 4), "frost"),
        [on_tile(0, 2), on_tile(1, 3), on_unit(7), on_unit(4)]
    );
    // Heal: the hurt knight beside it, never a tile or an enemy.
    assert_eq!(from(p(0, 4), "heal"), [on_unit(2)]);
    // From (0, 3), a step up: other tiles and only the elemental in range.
    assert_eq!(from(p(0, 3), "fire"), [on_unit(7)]);
    assert_eq!(
        from(p(0, 3), "frost"),
        [
            on_tile(0, 1),
            on_tile(0, 2),
            on_tile(1, 2),
            on_tile(1, 3),
            on_tile(2, 3),
            on_unit(7)
        ]
    );
    assert_eq!(from(p(0, 3), "heal"), []);
    // A tile it can't move to, or another unit's spell: nothing.
    assert_eq!(from(p(13, 0), "fire"), []);
    assert_eq!(cast_targets(&s, UnitId(1), p(3, 5), spell("fire")), []);
    // Where a target is: a unit's tile, or the tile.
    assert_eq!(target_pos(&s, on_unit(7)), Some(p(1, 4)));
    assert_eq!(target_pos(&s, on_tile(1, 5)), Some(p(1, 5)));
    assert_eq!(target_pos(&s, on_unit(99)), None);
    assert_eq!(
        cast(&sid("fire"), on_tile(1, 5)),
        UnitAction::Cast {
            spell: sid("fire"),
            target: on_tile(1, 5),
            active: None
        }
    );
}

#[test]
fn the_list_has_every_learned_spell_dimmed_without_a_target() {
    let c = ctx();
    let s = field(&c, 0);
    let choices = spell_choices(&s, MAGE, p(0, 4));
    let ids: Vec<&str> = choices.iter().map(|c| c.spell.0.as_str()).collect();
    assert_eq!(ids, ["fire", "frost", "heal"]);
    assert_eq!(choices[0].targets.len(), 4);
    assert!(choices[2].targets.is_empty(), "nobody is hurt");
    let menu = spell_menu(&s, MAGE, &choices);
    let lines: Vec<(&str, bool)> = menu
        .items()
        .iter()
        .map(|i| (i.label.as_str(), i.enabled))
        .collect();
    assert_eq!(
        lines,
        [
            ("Fire   10/10  Mt5 Hit90 Rng1-2", true),
            ("Frost  10/10  Mt4 Hit95 Rng1-2", true),
            ("Heal    8/8   HP+10 Rng1", false),
        ]
    );
    // It opens on the equipped spell (Fire), if it can be cast.
    assert_eq!(menu.focus(), 0);
    let mut units = s.units().to_vec();
    units[7].loadout.equipped = Some(Equipped::Spell(sid("frost")));
    units[7].spells.uses_left.insert(sid("fire"), 0);
    let rout = Objective::Rout { turn_limit: None };
    let s2 = battle_with(&c, s.map().clone(), units, rout);
    // (A battle's start refills every spell.)
    assert_eq!(uses(&s2, "fire"), 10);
    let choices2 = spell_choices(&s2, MAGE, p(0, 4));
    assert_eq!(spell_menu(&s2, MAGE, &choices2).focus(), 1);
    // The equipped spell with nothing to be cast on: the first that has.
    let far = spell_choices(&s2, MAGE, p(0, 3));
    assert_eq!(far[1].targets.len(), 6);
    let mut none = far.clone();
    none[1].targets.clear();
    assert_eq!(spell_menu(&s2, MAGE, &none).focus(), 0);
    // Uses are the unit's, name padded to `name_w`.
    let heal = c.content.spells.get(&sid("heal")).unwrap();
    assert_eq!(spell_label(heal, 3, 6), "Heal     3/8   HP+10 Rng1");
    // Who has the menu, and when it is enabled.
    assert!(knows_spells(&s, MAGE));
    assert!(!knows_spells(&s, UnitId(1)));
    assert!(!knows_spells(&s, UnitId(99)));
    assert!(spell_choices(&s, UnitId(1), p(3, 5)).is_empty());
    assert!(spell_choices(&s, UnitId(99), p(3, 5)).is_empty());
    assert!(can_cast(&choices));
    assert!(!can_cast(&none[1..2]));
    assert!(!can_cast(&[]));
}

#[test]
fn magic_is_listed_for_casters_and_is_the_default_only_against_an_enemy() {
    let c = ctx();
    let menu = |s: &BattleState, unit: UnitId| {
        let sel = Selection::new(s, unit).unwrap();
        match open_menu(sel, s) {
            Mode::ActionMenu { menu, entries, .. } => (menu, entries),
            m => panic!("{m:?}"),
        }
    };
    let magic = |s: &BattleState| {
        let (menu, entries) = menu(s, MAGE);
        let at = entries.iter().position(|&e| e == MenuEntry::Magic).unwrap();
        (menu.items()[at].enabled, entries[menu.focus()])
    };
    // An enemy in reach: Magic is where the menu opens.
    let s = field(&c, 0);
    let (_, entries) = menu(&s, MAGE);
    assert_eq!(
        entries,
        [
            MenuEntry::Attack,
            MenuEntry::Magic,
            MenuEntry::Item,
            MenuEntry::Equip,
            MenuEntry::Wait
        ]
    );
    assert_eq!(MenuEntry::Magic.label(), "Magic");
    assert_eq!(magic(&s), (true, MenuEntry::Magic));
    assert!(reaches_an_enemy(&s, &spell_choices(&s, MAGE, p(0, 4))));
    // The Quick Battle's start: only a forest to burn, so Magic is enabled
    // but the menu opens on Wait.
    let quick = quick_battle(&c.content).unwrap();
    assert_eq!(magic(&quick), (true, MenuEntry::Wait));
    assert!(!reaches_an_enemy(
        &quick,
        &spell_choices(&quick, MAGE, p(3, 6))
    ));
    // An ally to heal is no enemy either.
    let (map, mut units) = quick_units(&c);
    units[0].hp -= 5;
    let hurt = battle_with(&c, map, units, Objective::Rout { turn_limit: None });
    let choices = spell_choices(&hurt, MAGE, p(3, 6));
    assert_eq!(choices[2].targets, [on_unit(1)]);
    assert!(!reaches_an_enemy(&hurt, &choices));
    assert_eq!(magic(&hurt), (true, MenuEntry::Wait));
    // Nothing to cast on from (5, 6): listed, dimmed.
    let mut sel = Selection::new(&quick, MAGE).unwrap();
    sel.path = vec![p(3, 6), p(4, 6), p(5, 6)];
    assert!(menu_entries(&sel, &quick).contains(&MenuEntry::Magic));
    let Mode::ActionMenu { menu: m, .. } = open_menu(sel, &quick) else {
        panic!("no menu");
    };
    assert!(!m.items()[1].enabled);
    // No spells: no Magic.
    let (_, lord) = menu(&quick, UnitId(1));
    assert!(!lord.contains(&MenuEntry::Magic));
}

#[test]
fn spell_list_snapshot() {
    let mut c = ctx();
    let state = field(&c, 0);
    let s = spell_list(&mut c, state);
    assert_eq!(s.help(&c), "arrows choose · f cast · d back");
    // The mage is drawn where it stands, with no cursor on the map.
    assert_eq!(s.mode().drawn_pos(MAGE), Some(p(0, 4)));
    assert_eq!(s.mode().selection().map(|sel| sel.unit), Some(MAGE));
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

// ---- Casting at an enemy ---------------------------------------------------------

#[test]
fn fire_on_the_elemental_shows_the_forecast_and_casts() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    // The cursor is on the first target, the elemental, with the forecast
    // and the list of spell actives (the Mage's Overcast).
    assert_eq!(targeting(&s).target(), on_unit(7));
    assert_eq!(s.cursor().pos, p(1, 4));
    let t = targeting(&s).forecast().expect("a forecast");
    assert_eq!(t.with, Equipped::Spell(sid("fire")));
    assert_eq!(t.target(), ELEMENTAL);
    assert!(t.has_list());
    assert!(t.preview.forecast.attacker.effective);
    assert_eq!(t.preview.forecast.attacker.affinity, Some(Affinity::Weak));
    assert_eq!(
        s.help(&c),
        "Left/Right target · Up/Down art · f cast · d back"
    );
    assert!(!s.mode().picks_on_map(), "the forecast stays on Confirm");
    let buf = render(&s, &c);
    // The spell and its uses, where a weapon's durability goes; `!` on the
    // strike; no marker of its own for Weak.
    assert_eq!(text(&buf, LEFT_X, NAME_ROW + 1, 8), "Fire");
    assert_eq!(text(&buf, LEFT_X, NAME_ROW + 2, 8), "10/10");
    assert_eq!(text(&buf, LEFT_X + 6, STRIKE_ROW, 1), "!");
    assert_eq!(text(&buf, LEFT_X, AFFINITY_ROW, 8), "");
    // The list's border names the spell and its uses.
    assert!(shows(&buf, " Fire 10/10 "));
    assert!(shows(&buf, "Overcast"));
    // Both enemies are tinted as attack targets; the forests show what
    // they would become.
    let attack = c.palette.get(UiColor::AttackRange);
    let plain = render(&BattleScreen::new(s.state().clone()), &c);
    for (x, y) in [(1, 4), (2, 4)] {
        let before = tile(&plain, x, y).2;
        assert_eq!(tile(&buf, x, y).2, before.lerp(attack, OVERLAY_BLEND));
    }
    for (x, y) in [(1, 5), (0, 6)] {
        assert_eq!(tile(&plain, x, y).0, "♣♣");
        assert_eq!(tile(&buf, x, y).0, "**");
    }
    // Up and down move through the list, not the targets.
    step(&mut s, &mut c, &[Action::CursorDown]);
    assert_eq!(targeting(&s).target(), on_unit(7));
    let overcast = Some(SkillId::new("overcast"));
    assert_eq!(targeting(&s).forecast().unwrap().preview.active, overcast);
    step(&mut s, &mut c, &[Action::CursorUp]);
    assert_eq!(targeting(&s).forecast().unwrap().preview.active, None);
    // Confirm casts: the combat plays, a use is spent, the mage has acted.
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Combat(_)), "{:?}", s.mode());
    assert_eq!(uses(s.state(), "fire"), 9);
    assert!(s.state().unit(MAGE).unwrap().acted);
    assert!(s.state().unit(ELEMENTAL).is_none_or(|u| u.hp < u.stats.hp));
}

#[test]
fn an_active_chosen_in_the_list_is_cast_with_the_spell() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    step(&mut s, &mut c, &[Action::CursorDown]);
    let cmd = targeting(&s).command();
    assert_eq!(
        cmd,
        Command::Act {
            unit: MAGE,
            dest: p(0, 4),
            action: UnitAction::Cast {
                spell: sid("fire"),
                target: on_unit(7),
                active: Some(SkillId::new("overcast")),
            },
        }
    );
    // The chosen line stays on the next enemy; on a tile there is no list.
    step(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(targeting(&s).target(), on_unit(4));
    let active = |s: &BattleScreen| targeting(s).forecast().unwrap().preview.active.clone();
    assert_eq!(active(&s), Some(SkillId::new("overcast")));
    step(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(targeting(&s).target(), on_tile(1, 5));
    assert!(targeting(&s).forecast().is_none());
    // Overcast costs an extra use.
    step(&mut s, &mut c, &[Action::CursorLeft, Action::Confirm]);
    assert_eq!(uses(s.state(), "fire"), 8);
}

#[test]
fn down_and_up_walk_a_longer_list_of_actives_in_order() {
    let mut c = ctx();
    let plain = field(&c, 0);
    // The mage also knows the Mystic's Siphon: Attack, Overcast, Siphon.
    let mut units = plain.units().to_vec();
    units[7].learned_skills.insert(SkillId::new("siphon"));
    let rout = Objective::Rout { turn_limit: None };
    let state = battle_with(&c, plain.map().clone(), units, rout);
    let mut s = casting(&mut c, state, 0);
    let mut press = |a: Action| {
        step(&mut s, &mut c, &[a]);
        let chosen = targeting(&s).forecast().unwrap().preview.active.clone();
        chosen.map(|id| id.0)
    };
    assert_eq!(press(Action::CursorDown).as_deref(), Some("overcast"));
    assert_eq!(press(Action::CursorDown).as_deref(), Some("siphon"));
    assert_eq!(press(Action::CursorDown), None);
    assert_eq!(press(Action::CursorUp).as_deref(), Some("siphon"));
    assert_eq!(press(Action::CursorUp).as_deref(), Some("overcast"));
}

/// The forecast panel's text and colours, with the affinity marker.
fn forecast_snapshot(c: &mut Ctx, state: BattleState, line: usize) -> String {
    let s = casting(c, state, line);
    assert_eq!(targeting(&s).target(), on_unit(7));
    render(&s, c).to_snapshot(&c.palette)
}

#[test]
fn forecast_of_fire_on_a_weak_target_snapshot() {
    let mut c = ctx();
    let state = field(&c, 0);
    assert_snapshot!(forecast_snapshot(&mut c, state, 0));
}

#[test]
fn forecast_of_frost_on_an_absorbing_target_snapshot() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 1);
    // Frost's targets start with its tiles: the elemental is the third.
    step(&mut s, &mut c, &[Action::NextUnit, Action::NextUnit]);
    assert_eq!(targeting(&s).target(), on_unit(7));
    let t = targeting(&s).forecast().unwrap();
    let healed = t.preview.forecast.attacker.damage;
    assert_eq!(t.preview.forecast.attacker.affinity, Some(Affinity::Absorb));
    let buf = render(&s, &c);
    assert_eq!(
        text(&buf, LEFT_X, AFFINITY_ROW, 10),
        format!("heals {healed}")
    );
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn forecast_of_fire_on_a_resisting_target_snapshot() {
    let mut c = ctx();
    // No class resists yet: the elemental resists Fire here.
    let class = ClassId("frost_elemental".into());
    c.content
        .classes
        .classes
        .get_mut(&class)
        .unwrap()
        .affinities = vec![(Element::Fire, Affinity::Resist)];
    let state = field(&c, 0);
    let s = casting(&mut c, state, 0);
    let t = targeting(&s).forecast().unwrap();
    assert_eq!(t.preview.forecast.attacker.affinity, Some(Affinity::Resist));
    let buf = render(&s, &c);
    assert_eq!(text(&buf, LEFT_X, AFFINITY_ROW, 10), "(resist)");
    assert_eq!(text(&buf, LEFT_X + 6, STRIKE_ROW, 1), "");
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

// ---- Casting on a tile -----------------------------------------------------------

#[test]
fn one_cursor_steps_through_enemies_and_tiles_both_ways() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    let mut visit = |a: Action| {
        step(&mut s, &mut c, &[a]);
        (targeting(&s).target(), s.cursor().pos)
    };
    assert_eq!(visit(Action::CursorRight), (on_unit(4), p(2, 4)));
    assert_eq!(visit(Action::NextUnit), (on_tile(1, 5), p(1, 5)));
    // On a tile, up and down step through the targets too.
    assert_eq!(visit(Action::CursorDown), (on_tile(0, 6), p(0, 6)));
    assert_eq!(visit(Action::CursorDown), (on_unit(7), p(1, 4)));
    assert_eq!(visit(Action::PrevUnit), (on_tile(0, 6), p(0, 6)));
    assert_eq!(visit(Action::CursorUp), (on_tile(1, 5), p(1, 5)));
    assert_eq!(visit(Action::CursorLeft), (on_unit(4), p(2, 4)));
    // Other keys do nothing.
    assert_eq!(visit(Action::Info), (on_unit(4), p(2, 4)));
    assert_eq!(visit(Action::EndTurn), (on_unit(4), p(2, 4)));
}

#[test]
fn a_tile_shows_what_it_becomes_and_is_picked_on_the_map() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    step(&mut s, &mut c, &[Action::NextUnit, Action::NextUnit]);
    assert_eq!(targeting(&s).target(), on_tile(1, 5));
    let line = targeting(&s).preview(s.state());
    assert_eq!(line.as_deref(), Some("Forest → Burning (1 round)"));
    assert_eq!(s.help(&c), "arrows next target · f cast · d back");
    // A pick on the map: the Select key's, once it has a key.
    assert!(s.mode().picks_on_map());
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 1, 30, 40), "Forest → Burning (1 round)");
    // The side panel is the hover panel again, on the tile.
    assert!(shows(&buf, "Forest"));
    assert!(!shows(&buf, " Forecast "));
    // The tiles it can burn are drawn burning, in the fire colours.
    let fire = c.palette.lookup("fire").unwrap();
    let fire_bg = c.palette.lookup("fire_bg").unwrap();
    assert_eq!(tile(&buf, 1, 5), ("**".to_owned(), fire, fire_bg));
    assert_eq!(tile(&buf, 0, 6).0, "**");
    let changes = targeting(&s).tile_changes(s.state());
    let burning = c.content.terrain.display.id_of("burning").unwrap();
    assert_eq!(changes, [(p(1, 5), burning), (p(0, 6), burning)]);
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn frost_names_the_ice_and_heal_changes_no_tile() {
    let mut c = ctx();
    let state = field(&c, 18);
    let s = casting(&mut c, state.clone(), 1);
    assert_eq!(targeting(&s).target(), on_tile(0, 2));
    let preview = |s: &BattleScreen| targeting(s).preview(s.state());
    assert_eq!(preview(&s).as_deref(), Some("Sea → Ice"));
    let ice = c.content.terrain.display.id_of("ice").unwrap();
    assert_eq!(
        targeting(&s).tile_changes(s.state()),
        [(p(0, 2), ice), (p(1, 3), ice)]
    );
    let buf = render(&s, &c);
    assert_eq!(tile(&buf, 0, 2).0, "▒▒");
    assert_eq!(tile(&buf, 1, 3).0, "▒▒");
    let mut s = s;
    step(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(preview(&s).as_deref(), Some("Water → Ice"));
    // On an enemy the forecast says it all: no line.
    step(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(targeting(&s).target(), on_unit(7));
    assert_eq!(preview(&s), None);
    let heal = casting(&mut c, state, 2);
    assert!(targeting(&heal).tile_changes(heal.state()).is_empty());
    assert!(targeting(&heal).forecast().is_none());
}

#[test]
fn casting_fire_on_a_forest_sets_it_burning_with_a_flash() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    step(&mut s, &mut c, &[Action::NextUnit, Action::NextUnit]);
    assert_eq!(terrain(&c, s.state(), 1, 5), "forest");
    step(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.mode(), &Mode::default());
    assert_eq!(terrain(&c, s.state(), 1, 5), "burning");
    assert_eq!(uses(s.state(), "fire"), 9);
    let mage = s.state().unit(MAGE).unwrap();
    assert!(mage.acted);
    assert_eq!(mage.loadout.equipped, Some(Equipped::Spell(sid("fire"))));
    // The tile flashes: its background tinted towards its glyph colour,
    // fading out over the flash time.
    assert_eq!(
        s.flashes(),
        [TerrainFlash {
            pos: p(1, 5),
            t: 0.0
        }]
    );
    let fire = c.palette.lookup("fire").unwrap();
    let fire_bg = c.palette.lookup("fire_bg").unwrap();
    let buf = render(&s, &c);
    let lit = fire_bg.lerp(fire, OVERLAY_BLEND);
    assert_eq!(tile(&buf, 1, 5), ("**".to_owned(), fire, lit));
    // The other forest is a forest again.
    assert_eq!(tile(&buf, 0, 6).0, "♣♣");
    wait(&mut s, &mut c, TERRAIN_FLASH_S / 2.0);
    let half = fire_bg.lerp(fire, OVERLAY_BLEND / 2.0);
    assert_eq!(tile(&render(&s, &c), 1, 5).2, half);
    wait(&mut s, &mut c, TERRAIN_FLASH_S / 2.0);
    assert!(s.flashes().is_empty());
    assert_eq!(
        tile(&render(&s, &c), 1, 5),
        ("**".to_owned(), fire, fire_bg)
    );
}

#[test]
fn a_flash_fades_from_the_overlay_blend_to_nothing() {
    let at = |t| TerrainFlash { pos: p(0, 0), t }.strength();
    assert!((at(0.0) - OVERLAY_BLEND).abs() < 1e-6);
    assert!((at(TERRAIN_FLASH_S / 4.0) - OVERLAY_BLEND * 0.75).abs() < 1e-6);
    assert!(at(TERRAIN_FLASH_S).abs() < 1e-6);
    assert!(at(TERRAIN_FLASH_S * 3.0).abs() < 1e-6);
    assert!((TERRAIN_FLASH_S - 0.4).abs() < f32::EPSILON);
    // A flash off the view draws nothing (and nothing breaks).
    let c = ctx();
    let mut s = BattleScreen::new(field(&c, 0));
    let before = render(&s, &c);
    s.flashes.push(TerrainFlash {
        pos: p(500, 500),
        t: 0.0,
    });
    assert_eq!(
        render(&s, &c).to_snapshot(&c.palette),
        before.to_snapshot(&c.palette)
    );
}

/// Ends the phase of `s` until the player's next one starts.
fn next_turn(s: &mut BattleState) {
    for _ in 0..3 {
        s.apply(&Command::EndPhase).unwrap();
        if s.phase() == Phase::Player {
            return;
        }
    }
    panic!("the player phase never came back");
}

#[test]
fn burning_burnt_and_ice_tiles_snapshot() {
    let c = ctx();
    let mut state = field(&c, 0);
    let mut cast_on = |spell: &str, x, y| {
        let cmd = Command::Act {
            unit: MAGE,
            dest: p(0, 4),
            action: cast(&sid(spell), on_tile(x, y)),
        };
        state.apply(&cmd).unwrap();
        next_turn(&mut state);
    };
    // Turn 1: a forest burns, and is burnt by turn 2. Turn 2: the water
    // freezes, for good. Turn 3: the other forest burns.
    cast_on("fire", 1, 5);
    cast_on("frost", 1, 3);
    let burn = Command::Act {
        unit: MAGE,
        dest: p(0, 4),
        action: cast(&sid("fire"), on_tile(0, 6)),
    };
    state.apply(&burn).unwrap();
    assert_eq!(
        [(1, 5), (1, 3), (0, 6)].map(|(x, y)| terrain(&c, &state, x, y)),
        ["burnt", "ice", "burning"]
    );
    let s = BattleScreen::new(state);
    let buf = render(&s, &c);
    assert_eq!(tile(&buf, 1, 5).0, ",,");
    assert_eq!(tile(&buf, 1, 3).0, "▒▒");
    assert_eq!(tile(&buf, 0, 6).0, "**");
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn a_fire_burning_out_flashes_too() {
    let mut c = ctx();
    let state = field(&c, 0);
    let mut s = casting(&mut c, state, 0);
    step(
        &mut s,
        &mut c,
        &[Action::NextUnit, Action::NextUnit, Action::Confirm],
    );
    wait(&mut s, &mut c, 1.0);
    assert!(s.flashes().is_empty());
    // The enemy's phase passes; the fire burns out as the player's starts.
    s.apply(&Command::EndPhase);
    through_ai_phases(&mut s, &mut c, 0.01);
    assert_eq!(s.state().phase(), Phase::Player);
    assert_eq!(terrain(&c, s.state(), 1, 5), "burnt");
    let flashing: Vec<Pos> = s.flashes().iter().map(|f| f.pos).collect();
    assert_eq!(flashing, [p(1, 5)]);
}

// ---- Healing ---------------------------------------------------------------------

#[test]
fn heal_previews_the_hp_and_restores_it_with_a_popup() {
    let mut c = ctx();
    let state = field(&c, 18);
    let mut s = casting(&mut c, state, 2);
    assert_eq!(targeting(&s).target(), on_unit(2));
    assert_eq!(s.cursor().pos, p(0, 5));
    assert!(targeting(&s).forecast().is_none());
    // Heal 10 + Mag 6 (`magic.md`): 2 → 18.
    let line = targeting(&s).preview(s.state());
    assert_eq!(line.as_deref(), Some("Heal on Test Knight: HP 2 → 18"));
    assert_eq!(s.help(&c), "arrows next target · f cast · d back");
    assert!(s.mode().picks_on_map());
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 1, 30, 40), "Heal on Test Knight: HP 2 → 18");
    // The ally is tinted as a heal target.
    let plain = render(&BattleScreen::new(s.state().clone()), &c);
    let heal = c.palette.get(UiColor::HealRange);
    assert_eq!(
        tile(&buf, 0, 5).2,
        tile(&plain, 0, 5).2.lerp(heal, OVERLAY_BLEND)
    );
    step(&mut s, &mut c, &[Action::Confirm]);
    let state = s.state();
    assert_eq!(state.unit(UnitId(2)).unwrap().hp, 18);
    assert_eq!(uses(state, "heal"), 7);
    assert!(state.unit(MAGE).unwrap().acted);
    assert!(!state.unit(UnitId(2)).unwrap().acted);
    assert_eq!(
        s.popups(),
        [HealPopup {
            pos: p(0, 5),
            amount: 16,
            t: 0.0
        }]
    );
    // Capped at what is missing.
    let state = field(&c, 3);
    let capped = casting(&mut c, state, 2);
    let line = targeting(&capped).preview(capped.state());
    assert_eq!(line.as_deref(), Some("Heal on Test Knight: HP 17 → 20"));
}

// ---- Cancelling ------------------------------------------------------------------

#[test]
fn cancelling_at_each_level_leaves_the_battle_unchanged() {
    let mut c = ctx();
    let before = field(&c, 18);
    for line in 0..3 {
        let mut s = casting(&mut c, before.clone(), line);
        step(&mut s, &mut c, &[Action::NextUnit]);
        // Back to the spell list, on the spell, the cursor on the mage.
        step(&mut s, &mut c, &[Action::Cancel]);
        let Mode::SpellMenu { menu, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(menu.focus(), line);
        assert_eq!(s.cursor().pos, p(0, 4));
        // Back to the action menu, on Magic.
        step(&mut s, &mut c, &[Action::Cancel]);
        let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(entries[menu.focus()], MenuEntry::Magic);
        // Back to the selection, then to browsing.
        step(&mut s, &mut c, &[Action::Cancel]);
        assert!(matches!(s.mode(), Mode::Selected(_)), "{:?}", s.mode());
        step(&mut s, &mut c, &[Action::Cancel]);
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.state(), &before);
        assert_eq!(s.history().len(), 0);
        assert!(s.flashes().is_empty());
    }
}

#[test]
fn the_menus_sound_like_the_others() {
    let c = ctx();
    let s = field(&c, 0);
    let sel = Selection::new(&s, MAGE).unwrap();
    let action_menu = open_menu(sel.clone(), &s);
    let choices = spell_choices(&s, MAGE, p(0, 4));
    let menu = spell_menu(&s, MAGE, &choices);
    let list = Mode::SpellMenu {
        sel: sel.clone(),
        menu: menu.clone(),
        choices: choices.clone(),
    };
    let t = CastTargeting::new(&s, sel.clone(), menu.clone(), choices.clone(), 0).unwrap();
    let aim = Mode::CastTarget(Box::new(t));
    // Any key that goes deeper sounds as Select, back out as Cancel.
    let sound = |from: &Mode, to: &Mode| step_sound(Action::Info, from, to, &Effect::None);
    assert_eq!(sound(&action_menu, &list), Some(MenuSound::Select));
    assert_eq!(sound(&list, &aim), Some(MenuSound::Select));
    assert_eq!(sound(&aim, &list), Some(MenuSound::Cancel));
    assert_eq!(sound(&list, &action_menu), Some(MenuSound::Cancel));
    // A list without targets can't be aimed.
    let mut none = choices;
    none[0].targets.clear();
    assert_eq!(
        CastTargeting::new(&s, sel.clone(), menu.clone(), none, 0),
        None
    );
    assert_eq!(CastTargeting::new(&s, sel, menu, vec![], 0), None);
}

// ---- Equip and the info screen ---------------------------------------------------

#[test]
fn the_equip_menu_lists_attack_spells_after_the_weapons() {
    let mut c = ctx();
    let state = field(&c, 0);
    let choices = equip_choices(&state, MAGE);
    let listed: Vec<(Equipped, bool)> =
        choices.iter().map(|c| (c.what.clone(), c.usable)).collect();
    // Fire and Frost; Heal is no attack.
    assert_eq!(
        listed,
        [
            (Equipped::Spell(sid("fire")), true),
            (Equipped::Spell(sid("frost")), true)
        ]
    );
    assert!(can_equip(&choices));
    let menu = equip_menu(&state, MAGE, &choices);
    let lines: Vec<&str> = menu.items().iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        lines,
        [
            "* Fire   Mt 5 Hit 90 Crit 0 Wt 0 Rng1-2 10/10",
            "  Frost  Mt 4 Hit 95 Crit 0 Wt 0 Rng1-2 10/10",
        ]
    );
    assert_eq!(menu.focus(), 0);
    assert!(menu.items().iter().all(|i| i.suffix.is_none()));
    // Choosing Frost equips it, for free: the action menu is back on Equip.
    let mut s = mage_menu(&mut c, state);
    choose(&mut s, &mut c, MenuEntry::Equip);
    step(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    let mage = s.state().unit(MAGE).unwrap();
    assert_eq!(mage.loadout.equipped, Some(Equipped::Spell(sid("frost"))));
    assert!(!mage.acted);
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Equip);
    // Now Frost is marked and focused.
    let again = equip_choices(s.state(), MAGE);
    let menu = equip_menu(s.state(), MAGE, &again);
    assert_eq!(menu.focus(), 1);
    assert!(menu.items()[1].label.starts_with("* Frost"));
    assert!(menu.items()[0].label.starts_with("  Fire"));
}

#[test]
fn the_equip_menu_pads_names_and_dims_a_spell_out_of_uses() {
    let c = ctx();
    let (map, mut units) = quick_units(&c);
    // The mage with the lord's iron sword in its first slot.
    units[7].loadout.weapons[0] = units[0].loadout.weapons[0].clone();
    units[7]
        .weapon_ranks
        .insert(trpg_core::WeaponKind::Sword, trpg_core::WeaponRank::E);
    let rout = Objective::Rout { turn_limit: None };
    let state = battle_with(&c, map, units, rout);
    let choices = equip_choices(&state, MAGE);
    let listed: Vec<Equipped> = choices.iter().map(|c| c.what.clone()).collect();
    assert_eq!(
        listed,
        [
            Equipped::Weapon(0),
            Equipped::Spell(sid("fire")),
            Equipped::Spell(sid("frost"))
        ]
    );
    let menu = equip_menu(&state, MAGE, &choices);
    let lines: Vec<&str> = menu.items().iter().map(|i| i.label.as_str()).collect();
    // Names are padded to the longest, the sword's; Fire is still what it
    // has equipped, so the list opens there.
    assert_eq!(
        lines,
        [
            "  Iron Sword  Mt 5 Hit 90 Crit 0 Wt 2 Rng1 20/20",
            "* Fire        Mt 5 Hit 90 Crit 0 Wt 0 Rng1-2 10/10",
            "  Frost       Mt 4 Hit 95 Crit 0 Wt 0 Rng1-2 10/10",
        ]
    );
    assert_eq!(menu.focus(), 1);
    // A unit that isn't there has no lines to show.
    let ghost = equip_menu(&state, UnitId(99), &choices);
    assert!(ghost.items().iter().all(|i| i.label.is_empty()));
    assert!(equip_choices(&state, UnitId(99)).is_empty());
    // Fire with a single use, spent on the forest: dimmed in both lists,
    // and with Frost alone there is nothing to choose in Equip.
    let mut c = ctx();
    c.content.spells.spells.get_mut(&sid("fire")).unwrap().uses = 1;
    let mut state = field(&c, 0);
    assert!(can_equip(&equip_choices(&state, MAGE)));
    let burn = Command::Act {
        unit: MAGE,
        dest: p(0, 4),
        action: cast(&sid("fire"), on_tile(1, 5)),
    };
    state.apply(&burn).unwrap();
    next_turn(&mut state);
    assert_eq!(uses(&state, "fire"), 0);
    let choices = equip_choices(&state, MAGE);
    let usable: Vec<bool> = choices.iter().map(|c| c.usable).collect();
    assert_eq!(usable, [false, true]);
    assert!(!can_equip(&choices));
    let menu = equip_menu(&state, MAGE, &choices);
    assert_eq!(
        menu.items()[0].label,
        "* Fire   Mt 5 Hit 90 Crit 0 Wt 0 Rng1-2  0/1"
    );
    assert!(!menu.items()[0].enabled);
    let spells = spell_choices(&state, MAGE, p(0, 4));
    assert!(spells[0].targets.is_empty());
    let list = spell_menu(&state, MAGE, &spells);
    assert_eq!(list.items()[0].label, "Fire    0/1   Mt5 Hit90 Rng1-2");
    assert!(!list.items()[0].enabled);
    assert_eq!(list.focus(), 1, "the equipped spell can't be cast");
}

#[test]
fn the_info_screen_marks_the_equipped_spell_and_lists_affinities() {
    let mut c = ctx();
    let mut s = BattleScreen::new(field(&c, 0));
    s.cursor.jump(p(0, 4));
    step(&mut s, &mut c, &[Action::Info]);
    let buf = render(&s, &c);
    let row_of = |buf: &GlyphBuffer, what: &str| {
        (0..i32::from(CONSOLE_H))
            .map(|y| text(buf, 0, y, i32::from(CONSOLE_W)))
            .find(|row| row.contains(what))
            .unwrap_or_else(|| panic!("no {what}"))
    };
    // Fire is equipped; every spell shows its uses.
    assert!(row_of(&buf, "Fire").contains("E Fire"));
    assert!(row_of(&buf, "Fire").ends_with("10/10 ║"));
    assert!(!row_of(&buf, "Frost").contains("E Frost"));
    assert!(row_of(&buf, "Heal").contains("8/8"));
    // A mage has no affinities: no heading.
    assert!(!shows(&buf, "Affinities"));
    // The elemental: its affinities, under its (empty) weapon ranks.
    s.cursor.jump(p(1, 4));
    step(&mut s, &mut c, &[Action::Cancel]);
    s.cursor.jump(p(1, 4));
    step(&mut s, &mut c, &[Action::Info]);
    let buf = render(&s, &c);
    assert!(shows(&buf, "Frost Elemental"));
    assert!(row_of(&buf, "Affinities").starts_with("║  Affinities"));
    assert!(row_of(&buf, "Absorb").starts_with("║  Ice       Absorb"));
    assert!(row_of(&buf, "Weak").starts_with("║  Fire      Weak"));
    assert!(row_of(&buf, "Frost   ").contains("E Frost"));
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

// ---- Harness: the whole flow with the default keys --------------------------------

/// The glyphs of tile `(x, y)` on the harness's screen.
fn shown_tile(h: &Harness, x: i32, y: i32) -> String {
    let (cx, cy) = cell(x, y);
    text(h.game().buffer(), cx, cy, 2)
}

#[test]
fn harness_fire_on_a_forest_burns_it_and_it_is_burnt_next_turn() {
    let c = ctx();
    let quick = quick_battle(&c.content).unwrap();
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(quick)));
    // The cursor starts on the lord: down to the mage, select it, stay.
    h.keys("Down f f");
    // Up from Wait past Equip to Magic (Item is dimmed: nobody is hurt);
    // Fire; the forest at (1, 6) is its one target.
    h.keys("Up Up f f");
    assert!(shows(h.game().buffer(), "Forest → Burning (1 round)"));
    assert_eq!(shown_tile(&h, 1, 6), "**");
    h.keys("f");
    // (The EXP bar of the cast plays out.)
    h.wait(3.0);
    assert_eq!(shown_tile(&h, 1, 6), "**");
    assert_eq!(shown_tile(&h, 0, 6), "♣♣");
    // End the turn (three units are still ready); the enemy plays.
    h.keys("Space f");
    let mut waited = 0;
    while shown_tile(&h, 1, 6) != ",," {
        h.wait(0.5);
        waited += 1;
        assert!(waited < 400, "the fire never burnt out");
    }
    // Burnt for good: the other forest is untouched.
    assert_eq!(shown_tile(&h, 0, 6), "♣♣");
}

#[test]
fn harness_heal_on_the_ally_beside_raises_its_hp_and_spends_a_use() {
    let c = ctx();
    let (map, mut units) = quick_units(&c);
    // The knight, right of the mage, 18 HP down.
    units[1].hp -= 18;
    let rout = Objective::Rout { turn_limit: None };
    let state = battle_with(&c, map, units, rout);
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    h.keys("Down f f");
    // Magic (past Equip; the pack is empty, so Item is dimmed), then down
    // from Fire past Frost (nothing to freeze) to Heal.
    h.keys("Up Up f Down f");
    assert!(shows(h.game().buffer(), "Heal on Test Knight: HP 2 → 18"));
    h.keys("f");
    assert!(shows(h.game().buffer(), "+16"));
    h.wait(3.0);
    // The cursor is still on the knight: its HP in the side panel. Then
    // the mage's uses, on its info screen.
    assert!(shows(h.game().buffer(), "HP 18/20"));
    h.keys("Left e");
    let info = h.game().buffer();
    assert!(shows(info, "Test Mage"));
    assert!(shows(info, "7/8"));
    assert!(shows(info, "10/10"));
}
