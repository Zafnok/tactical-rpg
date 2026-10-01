//! Tests of the class-choice screen, on the debug menu's test knight: a
//! Guard (mastered) with Def 49 of 50, a Rider record at class level 6 and
//! one of every seal.

use insta::assert_snapshot;
use trpg_core::{ItemId, WeaponKind, WeaponRank};

use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::screen::tests::ctx;
use Action::{Cancel, Confirm, CursorLeft, CursorRight};

fn demo(c: &Ctx, kind: ChangeKind) -> ClassChangeScreen {
    ClassChangeScreen::demo(c, kind).expect("the test knight")
}

fn step(s: &mut ClassChangeScreen, c: &mut Ctx, actions: &[Action]) -> Transition {
    s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]))
}

fn render(s: &ClassChangeScreen, c: &Ctx) -> GlyphBuffer {
    let stale = Cell::new(
        'x',
        crate::color::Rgb::new(1, 2, 3),
        crate::color::Rgb::new(1, 2, 3),
    );
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(c, &mut buf);
    buf
}

/// The text of `w` cells of row `y` from column `x`, trimmed.
fn text(buf: &GlyphBuffer, x: i32, y: i32, w: i32) -> String {
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The text inside column `slot` (0 = leftmost shown) on row `row` of it.
fn cell(buf: &GlyphBuffer, slot: i32, row: i32) -> String {
    text(buf, COLUMNS_X + slot * COLUMN_W + 2, COLUMNS_Y + row, 20)
}

/// Whether some row of the screen contains `needle`.
fn shows(buf: &GlyphBuffer, needle: &str) -> bool {
    (0..i32::from(CONSOLE_H)).any(|y| text(buf, 0, y, i32::from(CONSOLE_W)).contains(needle))
}

fn classes(s: &ClassChangeScreen) -> Vec<&str> {
    s.options().iter().map(|o| o.class.0.as_str()).collect()
}

fn item(id: &str) -> ItemId {
    ItemId::new(id)
}

#[test]
fn a_promotion_offers_the_branches_with_what_promote_would_do() {
    let c = ctx();
    let s = demo(&c, ChangeKind::Promote);
    assert_eq!(s.name(), "class_change");
    assert_eq!(classes(&s), ["bulwark", "iron_rider"]);
    assert_eq!(s.focus(), 0);
    // Each preview is `promote` on a copy of the unit and the stock.
    for option in s.options() {
        let (mut unit, mut stock) = (s.unit().clone(), s.stock().clone());
        let made = promote(&mut unit, &option.class, &mut stock, &tables(&c));
        assert!(made.is_ok(), "{made:?}");
        assert_eq!(option.after.as_ref(), Ok(&unit), "{:?}", option.class);
    }
    // The screen's own unit and stock are untouched until a choice is made.
    assert_eq!(s.unit().class, ClassId("guard".into()));
    assert_eq!(s.stock().count(&item("tier_2_seal")), 1);
}

/// The knight: HP 28, Str 11, Mag 0, Dex 7, Spd 4, Def 49, Res 0. Bulwark
/// base `28 11 0 6 4 14 2` less Guard `20 7 0 4 2 9 0`; Iron Rider
/// `27 10 0 6 6 11 2`.
#[test]
fn the_columns_show_each_stat_before_and_after_and_mark_a_maxed_one() {
    let c = ctx();
    let s = demo(&c, ChangeKind::Promote);
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 2, 1, 40), "Promotion");
    assert_eq!(text(&buf, 2, 2, 40), "Test Knight  Guard  Lv 12");
    assert_eq!(text(&buf, 70, 2, 28), "Tier 2 Seal ×1");
    let column = |slot| (1..24).map(|row| cell(&buf, slot, row)).collect::<Vec<_>>();
    assert_eq!(
        column(0),
        [
            "Bulwark",
            "Tier 2",
            "",
            "Mov   4     armored",
            "Armored",
            "",
            "Weapons",
            "Spear    D → C",
            "Axe      E → D",
            "",
            "",
            "Active",
            "Fortify",
            "",
            "HP   28 → 36  +8",
            "Str  11 → 15  +4",
            "Mag   0",
            "Dex   7 → 9   +2",
            "Spd   4 → 6   +2",
            // Def +5 is cut at the hard ceiling (50).
            "Def  49 → 50  +1 MAX",
            "Res   0 → 2   +2",
            "",
            "",
        ]
    );
    assert_eq!(
        column(1),
        [
            "Iron Rider",
            "Tier 2",
            "",
            "Mov   4 → 6 mounted",
            "Mounted Armored",
            "",
            "Weapons",
            "Spear    D → C",
            "Axe      E → D",
            "Sword    E → D",
            "",
            "Active",
            "Trample",
            "",
            "HP   28 → 35  +7",
            "Str  11 → 14  +3",
            "Mag   0",
            "Dex   7 → 9   +2",
            "Spd   4 → 8   +4",
            "Def  49 → 50  +1 MAX",
            "Res   0 → 2   +2",
            "",
            "",
        ]
    );
    // Two columns only: nothing past them, no scroll marks.
    assert_eq!(text(&buf, COLUMNS_X + 2 * COLUMN_W, COLUMNS_Y, 48), "");
    assert_eq!(text(&buf, 0, MORE_ROW, 100), "");
    // The gain and the mark are highlighted; the focused column is boxed
    // double, the other single.
    let hi = c.palette.get(UiColor::TextHighlight);
    let at = |x: i32, row: i32| buf.get(COLUMNS_X + 2 + x, COLUMNS_Y + row).unwrap();
    assert_eq!((at(14, 15).glyph, at(14, 15).fg), ('+', hi));
    assert_eq!((at(17, 20).glyph, at(17, 20).fg), ('M', hi));
    assert_eq!(at(0, 15).fg, c.palette.get(UiColor::Text));
    assert_eq!(at(0, 17).fg, c.palette.get(UiColor::TextDim));
    assert_eq!(buf.get(COLUMNS_X, COLUMNS_Y).unwrap().glyph, '╔');
    assert_eq!(buf.get(COLUMNS_X + COLUMN_W, COLUMNS_Y).unwrap().glyph, '┌');
    assert_eq!(s.help(&c), "Left/Right class · f choose · d back");
}

#[test]
fn text_rows() {
    assert_eq!(stat_line(StatKind::Str, 9, 12, 50), "Str   9 → 12  +3");
    assert_eq!(stat_line(StatKind::Mag, 1, 1, 50), "Mag   1");
    assert_eq!(stat_line(StatKind::Def, 48, 50, 50), "Def  48 → 50  +2 MAX");
    // Already at the ceiling, or past it.
    assert_eq!(stat_line(StatKind::Res, 50, 50, 50), "Res  50 MAX");
    assert_eq!(stat_line(StatKind::Hp, 81, 81, 80), "HP   81 MAX");
    assert_eq!(stat_line(StatKind::Hp, 70, 79, 80), "HP   70 → 79  +9");
    assert_eq!(mov_line(5, 5), "Mov   5");
    assert_eq!(mov_line(4, 6), "Mov   4 → 6");
    let c = ctx();
    let mut unit = demo(&c, ChangeKind::Promote).unit().clone();
    unit.weapon_ranks.insert(WeaponKind::Spear, WeaponRank::A);
    let class = |id: &str| c.content.classes.get(&ClassId(id.into())).unwrap();
    // A rank at or above the start rank stays.
    assert_eq!(
        weapon_lines(&unit, class("iron_rider")),
        ["Spear    A", "Axe      E → D", "Sword    E → D"]
    );
    assert_eq!(weapon_lines(&unit, class("archmage")), Vec::<String>::new());
}

#[test]
fn choosing_asks_first_then_promotes_and_shows_the_bonus() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    // Left and right move between the columns, wrapping.
    step(&mut s, &mut c, &[CursorRight]);
    assert_eq!(s.focus(), 1);
    step(&mut s, &mut c, &[CursorRight]);
    assert_eq!(s.focus(), 0);
    step(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(s.focus(), 1);
    // Up and down do nothing.
    step(&mut s, &mut c, &[Action::CursorUp, Action::CursorDown]);
    assert_eq!((s.focus(), s.is_confirming()), (1, false));
    // Confirm asks; Cancel backs out, the cursor where it was.
    step(&mut s, &mut c, &[Confirm]);
    assert!(s.is_confirming());
    assert_eq!(s.help(&c), "f yes · d no");
    let buf = render(&s, &c);
    for line in [
        "║ Promote Test Knight to Iron Rider? ║",
        "║ Uses 1 Tier 2 Seal (1 in stock).   ║",
        "║ f yes / d no                       ║",
    ] {
        assert!(shows(&buf, line), "{line}");
    }
    // The cursor keys do nothing while it asks.
    step(&mut s, &mut c, &[CursorLeft]);
    assert_eq!((s.focus(), s.is_confirming()), (1, true));
    step(&mut s, &mut c, &[Cancel]);
    assert_eq!((s.focus(), s.is_confirming()), (1, false));
    assert_eq!(s.unit().class, ClassId("guard".into()));
    // Confirm twice makes the change.
    let before = s.unit().clone();
    let expected = s.options()[1].after.clone();
    let t = step(&mut s, &mut c, &[Confirm, Confirm]);
    assert!(matches!(t, Transition::None));
    assert_eq!(Ok(s.unit()), expected.as_ref());
    assert_eq!(s.unit().class, ClassId("iron_rider".into()));
    assert_eq!((s.unit().level, s.unit().exp), (before.level, before.exp));
    assert_eq!(s.stock().count(&item("tier_2_seal")), 0);
    assert_eq!(s.stock().count(&item("reclass_seal")), 1);
    // Its page: the level-up panel with the bonus, one stat at a time.
    let pages = s.progress().expect("pages").pages().to_vec();
    let [progress::Page::Promotion(page)] = pages.as_slice() else {
        panic!("{pages:?}")
    };
    assert_eq!(
        (page.from.as_str(), page.to.as_str()),
        ("Guard", "Iron Rider")
    );
    assert_eq!(page.before, [28, 11, 0, 7, 4, 49, 0]);
    assert_eq!(page.gains.0, [7, 3, 0, 2, 4, 1, 2]);
    assert_eq!(page.character.as_deref(), Some("test_knight"));
    assert_eq!(s.help(&c), "f skip · hold f fast");
    // A press reveals every stat, the next closes the page and the screen.
    let t = step(&mut s, &mut c, &[Confirm]);
    assert!(matches!(t, Transition::None));
    assert_eq!(s.help(&c), "f continue");
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 43, 7, 20), "PROMOTED!");
    assert_eq!(text(&buf, 43, 9, 20), "Test Knight");
    assert_eq!(text(&buf, 43, 10, 20), "Guard → Iron Rider");
    assert_eq!(text(&buf, 43, 12, 20), "HP   28 → 35  +7");
    assert_eq!(text(&buf, 43, 17, 20), "Def  49 → 50  +1");
    // Nothing else is on screen.
    assert_eq!(text(&buf, 0, 2, 100), "");
    let t = step(&mut s, &mut c, &[Cancel]);
    assert!(matches!(t, Transition::Pop), "{t:?}");
}

#[test]
fn the_pages_play_by_themselves_and_holding_confirm_speeds_them_up() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    step(&mut s, &mut c, &[Confirm, Confirm]);
    let frame = |dt, held: &[Action]| FrameInput::new(vec![], dt, held.to_vec());
    s.update(&mut c, &frame(0.1, &[]));
    let t = s.progress().map(Progress::time).unwrap_or_default();
    assert!((t - 0.1).abs() < 1e-5, "{t}");
    s.update(&mut c, &frame(0.1, &[Confirm]));
    let t = s.progress().map(Progress::time).unwrap_or_default();
    assert!((t - 0.5).abs() < 1e-5, "{t}");
    // Played to its end, the page still waits for a key.
    let t = s.update(&mut c, &frame(60.0, &[]));
    assert!(matches!(t, Transition::None));
    assert!(s.progress().is_some_and(Progress::page_played));
}

#[test]
fn cancel_closes_the_screen_with_nothing_changed() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    let (unit, stock) = (s.unit().clone(), s.stock().clone());
    let t = step(&mut s, &mut c, &[CursorRight, Cancel]);
    assert!(matches!(t, Transition::Pop));
    assert_eq!((s.unit(), s.stock()), (&unit, &stock));
}

#[test]
fn a_refused_class_is_dimmed_with_the_reason_and_cannot_be_chosen() {
    let mut c = ctx();
    let knight = demo(&c, ChangeKind::Promote).unit().clone();
    // No seal in stock.
    let mut s = ClassChangeScreen::new(&c, ChangeKind::Promote, knight.clone(), Stock::default());
    assert_eq!(
        s.options()[0].after,
        Err(ClassChangeError::NoSeal(SealKind::Tier(2)))
    );
    let buf = render(&s, &c);
    assert_eq!(cell(&buf, 0, 23), "No Tier 2 Seal");
    // Its stats stay as they are, dimmed like the rest.
    assert_eq!(cell(&buf, 0, 15), "HP   28");
    assert_eq!(cell(&buf, 0, 20), "Def  49");
    let dim = c.palette.get(UiColor::TextDim);
    let at = |x: i32, row: i32| buf.get(COLUMNS_X + 2 + x, COLUMNS_Y + row).unwrap();
    assert_eq!((at(0, 1).fg, at(0, 13).fg), (dim, dim));
    assert_eq!(at(0, 23).fg, c.palette.get(UiColor::HpLow));
    assert_eq!(text(&buf, 70, 2, 28), "Tier 2 Seal ×0");
    assert_eq!(s.help(&c), "Left/Right class · d back");
    step(&mut s, &mut c, &[Confirm]);
    assert!(!s.is_confirming());
    // Not mastered.
    let mut fresh = knight;
    fresh
        .class_records
        .insert(ClassId("guard".into()), ClassRecord::UNLOCKED);
    let mut stock = Stock::default();
    stock.add(item("tier_2_seal"));
    let s = ClassChangeScreen::new(&c, ChangeKind::Promote, fresh, stock);
    assert_eq!(cell(&render(&s, &c), 1, 23), "Class not mastered");
}

#[test]
fn a_unit_with_nowhere_to_go_sees_an_empty_screen() {
    let mut c = ctx();
    let mut top = demo(&c, ChangeKind::Promote).unit().clone();
    top.class = ClassId("bastion".into());
    let mut s = ClassChangeScreen::new(&c, ChangeKind::Promote, top, Stock::default());
    assert_eq!(s.options().len(), 0);
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 0, COLUMNS_Y + 1, 100), "No class to change to.");
    assert_eq!(s.help(&c), "d back");
    let t = step(&mut s, &mut c, &[CursorRight, Confirm]);
    assert!(matches!(t, Transition::None));
    assert!(!s.is_confirming());
    assert!(matches!(step(&mut s, &mut c, &[Cancel]), Transition::Pop));
}

#[test]
fn a_reclass_lists_every_class_the_seal_reaches_and_scrolls() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Reclass);
    // Tier 1 (but its own Guard), then what the mastered Guard promotes to.
    assert_eq!(
        classes(&s),
        [
            "archer",
            "brawler",
            "cleric",
            "mage",
            "raider",
            "rider",
            "swordsman",
            "bulwark",
            "iron_rider"
        ]
    );
    for option in s.options() {
        let (mut unit, mut stock) = (s.unit().clone(), s.stock().clone());
        let made = reclass(&mut unit, &option.class, &mut stock, &tables(&c));
        assert!(made.is_ok(), "{made:?}");
        assert_eq!(option.after.as_ref(), Ok(&unit), "{:?}", option.class);
    }
    let buf = render(&s, &c);
    assert_eq!(text(&buf, 2, 1, 40), "Class change");
    assert_eq!(text(&buf, 70, 2, 28), "Reclass Seal ×1");
    let names = |buf: &GlyphBuffer| (0..4).map(|slot| cell(buf, slot, 1)).collect::<Vec<_>>();
    assert_eq!(names(&buf), ["Archer", "Brawler", "Cleric", "Mage"]);
    assert_eq!(text(&buf, 0, MORE_ROW, 100), "more →");
    // A class never entered is `new`; no stat changes, the maxed one marked.
    assert_eq!(cell(&buf, 0, 2), "Tier 1   new");
    assert_eq!(cell(&buf, 0, 4), "Mov   4 → 5 foot");
    assert_eq!(cell(&buf, 0, 15), "HP   28");
    assert_eq!(cell(&buf, 0, 20), "Def  49");
    // Moving past the last column shown scrolls.
    for _ in 0..5 {
        step(&mut s, &mut c, &[CursorRight]);
    }
    assert_eq!(s.focus(), 5);
    let buf = render(&s, &c);
    assert_eq!(names(&buf), ["Cleric", "Mage", "Raider", "Rider"]);
    assert_eq!(
        text(&buf, 0, MORE_ROW, 100).replace("  ", ""),
        "← moremore →"
    );
    // The Rider's saved class level.
    assert_eq!(cell(&buf, 3, 2), "Tier 1   CL 6");
    assert_eq!(
        buf.get(COLUMNS_X + 3 * COLUMN_W, COLUMNS_Y).unwrap().glyph,
        '╔'
    );
    // One more: both marks still show, with two columns left to the right.
    step(&mut s, &mut c, &[CursorRight]);
    let buf = render(&s, &c);
    assert_eq!(names(&buf), ["Mage", "Raider", "Rider", "Swordsman"]);
    assert!(text(&buf, 0, MORE_ROW, 100).ends_with("more →"));
    // Wrapping to the first column scrolls back; and to the last.
    for _ in 0..3 {
        step(&mut s, &mut c, &[CursorRight]);
    }
    assert_eq!(s.focus(), 0);
    assert_eq!(cell(&render(&s, &c), 0, 1), "Archer");
    step(&mut s, &mut c, &[CursorLeft]);
    let buf = render(&s, &c);
    assert_eq!(names(&buf), ["Rider", "Swordsman", "Bulwark", "Iron Rider"]);
    assert_eq!(text(&buf, 0, MORE_ROW, 100), "← more");
    // No bonus by this road: a reclass never changes stats.
    assert_eq!(cell(&buf, 3, 15), "HP   28");
    assert_eq!(cell(&buf, 3, 2), "Tier 2   new");
}

#[test]
fn a_reclass_keeps_stats_and_shows_what_was_learned_and_stowed() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Reclass);
    let stats = s.unit().stats;
    step(&mut s, &mut c, &[CursorRight, CursorRight, CursorRight]);
    step(&mut s, &mut c, &[Confirm]);
    let buf = render(&s, &c);
    assert!(shows(&buf, "║ Change Test Knight to Mage?       ║"));
    assert!(shows(&buf, "║ Uses 1 Reclass Seal (1 in stock). ║"));
    step(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.unit().class, ClassId("mage".into()));
    // Only Mov follows the class.
    let mut expected = stats;
    expected.mov = 5;
    assert_eq!(s.unit().stats, expected);
    assert_eq!(s.stock().count(&item("reclass_seal")), 0);
    assert_eq!(s.stock().count(&item("tier_2_seal")), 1);
    // The Mage wears light armour only: the chain mail went to the stock.
    assert_eq!(s.stock().count(&item("chain_mail")), 1);
    let pages = s.progress().expect("pages").pages().to_vec();
    let [progress::Page::Class(page)] = pages.as_slice() else {
        panic!("{pages:?}")
    };
    assert_eq!(page.class, "Guard → Mage");
    assert!(page.changed);
    assert_eq!(page.learned, ["Fire"]);
    assert_eq!(page.stowed, ["Chain Mail"]);
    let buf = render(&s, &c);
    assert!(shows(&buf, "Test Knight  Guard → Mage"));
    assert!(shows(&buf, "Learned: Fire"));
    assert!(shows(&buf, "To stock: Chain Mail"));
    // The page waits for a key, then the screen closes.
    let idle = s.update(&mut c, &FrameInput::new(vec![], 60.0, vec![]));
    assert!(matches!(idle, Transition::None));
    assert!(matches!(step(&mut s, &mut c, &[Confirm]), Transition::Pop));
}

#[test]
fn sounds() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    let heard = |s: &mut ClassChangeScreen, c: &mut Ctx, action| {
        step(s, c, &[action]);
        let sounds: Vec<String> = c
            .audio
            .take()
            .into_iter()
            .map(|r| format!("{r:?}"))
            .collect();
        sounds.join(",")
    };
    let moved = heard(&mut s, &mut c, CursorRight);
    assert!(moved.contains("menu_move"), "{moved}");
    assert_eq!(heard(&mut s, &mut c, Action::CursorUp), "");
    let chosen = heard(&mut s, &mut c, Confirm);
    assert!(chosen.contains("menu_select"), "{chosen}");
    let back = heard(&mut s, &mut c, Cancel);
    assert!(back.contains("menu_cancel"), "{back}");
    heard(&mut s, &mut c, Confirm);
    let made = heard(&mut s, &mut c, Confirm);
    assert!(made.contains("menu_select"), "{made}");
    // A single column has nowhere to move to; a refused class is denied.
    let knight = demo(&c, ChangeKind::Promote).unit().clone();
    let mut none = ClassChangeScreen::new(&c, ChangeKind::Promote, knight, Stock::default());
    let denied = heard(&mut none, &mut c, Confirm);
    assert!(denied.contains("menu_cancel"), "{denied}");
    let mut lancer = demo(&c, ChangeKind::Promote).unit().clone();
    lancer.class = ClassId("duelist".into());
    let mut one = ClassChangeScreen::new(&c, ChangeKind::Promote, lancer, Stock::default());
    assert_eq!(one.options().len(), 1);
    // Nothing to pick between, and nothing to choose (no seal).
    assert_eq!(one.help(&c), "d back");
    assert_eq!(heard(&mut one, &mut c, CursorRight), "");
    assert_eq!(one.focus(), 0);
    let closed = heard(&mut one, &mut c, Cancel);
    assert!(closed.contains("menu_cancel"), "{closed}");
}

#[test]
fn the_demo_needs_its_test_character() {
    let mut c = ctx();
    c.content.characters.characters.clear();
    assert!(ClassChangeScreen::demo(&c, ChangeKind::Promote).is_none());
}

#[test]
fn promotion_choice_snapshot() {
    let c = ctx();
    let s = demo(&c, ChangeKind::Promote);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn promotion_question_snapshot() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    step(&mut s, &mut c, &[CursorRight, Confirm]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn promotion_result_snapshot() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Promote);
    step(&mut s, &mut c, &[Confirm, Confirm, Confirm]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn reclass_choice_snapshot() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Reclass);
    for _ in 0..5 {
        step(&mut s, &mut c, &[CursorRight]);
    }
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn reclass_result_snapshot() {
    let mut c = ctx();
    let mut s = demo(&c, ChangeKind::Reclass);
    step(&mut s, &mut c, &[CursorRight, CursorRight, CursorRight]);
    step(&mut s, &mut c, &[Confirm, Confirm]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}
