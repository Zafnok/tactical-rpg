//! Tests of Combat Arts in the attack flow (ticket 0414): the arts list
//! (arts and combat actives, dimmed when they can't be paid for), the
//! forecast with the chosen line, durability on the weapon lines, debuffs on
//! the info screen, and the playback banner for a boss's or a green unit's
//! art. The numbers are always `core`'s.

use insta::assert_snapshot;
use trpg_core::{
    ArtId, BattleState, ClassId, Command, EffectSource, Equipped, Event, Faction, Objective, Pos,
    SkillId, Unit, UnitId,
};

use super::BattleScreen;
use super::art_list::{Technique, playback_banner};
use super::forecast::{HP_ROW, LEFT_X, RIGHT_X, SKILL_ROW, STRIKE_ROW};
use super::mode::Mode;
use super::playback::{BOX, Playback, TIMINGS, draw_box};
use super::testing::{battle_with, quick_units};
use crate::FrameInput;
use crate::color::{Rgb, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::tests::ctx;
use crate::screen::{Ctx, Screen};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
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

/// Cells `x..x + w` of row `y`, trimmed.
fn text(buf: &GlyphBuffer, x: i32, y: i32, w: i32) -> String {
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Every row of the screen, joined.
fn screen_text(buf: &GlyphBuffer) -> String {
    (0..i32::from(CONSOLE_H))
        .map(|y| text(buf, 0, y, i32::from(CONSOLE_W)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The Quick Battle's units on a rout battle, `edit` applied to them first:
/// the lord at (6, 2) beside the raider (7, 1) and the brigand (8, 2), the
/// archer two tiles below the brigand at (8, 4).
fn battle(c: &Ctx, edit: impl FnOnce(&mut Vec<Unit>)) -> BattleState {
    let (map, mut units) = quick_units(c);
    units[0].pos = p(6, 2);
    units[2].pos = p(8, 4);
    edit(&mut units);
    battle_with(c, map, units, Objective::Rout { turn_limit: None })
}

/// The lord as a Swordsman (Sword D: Flowing Cut and Guard Break), its
/// iron sword at `durability`.
fn swordsman(c: &Ctx, durability: u32) -> BattleState {
    battle(c, |units| {
        units[0].class = ClassId("swordsman".to_owned());
        units[0].loadout.weapons[0]
            .as_mut()
            .unwrap()
            .durability_left = durability;
    })
}

/// The targeting state of `s`.
fn targeting(s: &BattleScreen) -> &super::attack::Targeting {
    match s.mode() {
        Mode::Targeting(t) => t,
        m => panic!("{m:?}"),
    }
}

/// The lord moved to (7, 2), attacking the brigand (unit 4) with its iron
/// sword.
fn lord_on_brigand(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::CursorRight, Action::Confirm],
    );
    wait(&mut s, c, 0.5);
    // Attack, then the iron sword; the raider is first, the brigand next.
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::Confirm, Action::CursorRight],
    );
    assert_eq!(targeting(&s).target(), UnitId(4));
    s
}

/// The archer attacking the brigand from where it stands, two tiles below.
fn archer_on_brigand(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    s.cursor.jump(p(8, 4));
    step(&mut s, c, &[Action::Confirm, Action::Confirm]);
    step(&mut s, c, &[Action::Confirm]);
    assert_eq!(targeting(&s).target(), UnitId(4));
    s
}

/// Moves the list down until the line named `name` is chosen.
fn choose(s: &mut BattleScreen, c: &mut Ctx, name: &str) {
    for _ in 0..8 {
        if targeting(s).technique().name(s.state()) == name {
            return;
        }
        step(s, c, &[Action::CursorDown]);
    }
    panic!("{name} can't be chosen");
}

/// The list's lines: name as shown, enabled, dimmed reason.
fn lines(s: &BattleScreen) -> Vec<(String, bool, Option<String>)> {
    targeting(s)
        .list
        .items()
        .iter()
        .map(|i| {
            let name = i.label.split("  ").next().unwrap_or("").to_owned();
            (
                name,
                i.enabled,
                i.suffix.clone().map(|(t, _)| t.trim().to_owned()),
            )
        })
        .collect()
}

/// The durability left on `unit`'s weapon in slot 0.
fn durability(state: &BattleState, unit: UnitId) -> u32 {
    let u = state.unit(unit).unwrap();
    u.loadout.weapon(0).unwrap().durability_left
}

/// The info screen of unit `id` of `state`, rendered.
fn info(c: &mut Ctx, state: &BattleState, id: UnitId) -> GlyphBuffer {
    let mut s = BattleScreen::new(state.clone());
    s.cursor.jump(state.unit(id).unwrap().pos);
    step(&mut s, c, &[Action::Info]);
    assert!(matches!(s.mode(), Mode::Info { .. }), "{:?}", s.mode());
    render(&s, c)
}

#[test]
fn guard_break_shows_cores_forecast_and_is_paid_when_attacking() {
    let mut c = ctx();
    let state = swordsman(&c, 20);
    let art = Technique::Art(ArtId::new("guard_break"));
    let expected = state
        .preview_attack(
            UnitId(1),
            p(7, 2),
            &art.action(UnitId(4), &Equipped::Weapon(0)),
        )
        .unwrap();
    let mut s = lord_on_brigand(&mut c, state);
    choose(&mut s, &mut c, "Guard Break");
    assert_eq!(targeting(&s).preview, expected);
    let buf = render(&s, &c);
    assert_eq!(text(&buf, LEFT_X, SKILL_ROW, 26), "Guard Break (20 → 16)");
    assert_eq!(text(&buf, LEFT_X + 5, HP_ROW + 1, 3), "100");
    assert_eq!(expected.forecast.attacker.hit, 100);
    assert_eq!(text(&buf, RIGHT_X + 2, STRIKE_ROW, 11), "no counter");
    assert_eq!(text(&buf, RIGHT_X + 5, HP_ROW + 1, 3), "--");
    // The help line names the list's keys.
    assert!(s.help(&c).contains("Up/Down art"), "{}", s.help(&c));
    // Attack: the playback, then the weapon has paid 4.
    step(&mut s, &mut c, &[Action::Confirm]);
    for _ in 0..40 {
        wait(&mut s, &mut c, 0.5);
        step(&mut s, &mut c, &[Action::Confirm]);
    }
    assert_eq!(durability(s.state(), UnitId(1)), 16);
    let after = s.state().clone();
    let buf = info(&mut c, &after, UnitId(1));
    let weapons = text(&buf, 56, 2, 43);
    assert!(weapons.ends_with("16/20"), "{weapons}");
}

#[test]
fn guard_break_with_three_left_goes_through_and_breaks_the_sword() {
    // Nick (0414 review): "it has 3 dur left and the art costs 5, ok, art
    // goes thru, then weapon is broken".
    let mut c = ctx();
    let state = swordsman(&c, 3);
    let mut s = lord_on_brigand(&mut c, state);
    let shown = lines(&s);
    let gb = shown.iter().find(|l| l.0 == "Guard Break").unwrap();
    assert_eq!((gb.1, gb.2.as_deref()), (true, None));
    choose(&mut s, &mut c, "Guard Break");
    let buf = render(&s, &c);
    assert_eq!(text(&buf, LEFT_X, SKILL_ROW, 26), "Guard Break (3 → 0)");
    // Fought with the unbroken sword: the same hit as at 20/20.
    assert_eq!(text(&buf, LEFT_X + 5, HP_ROW + 1, 3), "100");
    step(&mut s, &mut c, &[Action::Confirm]);
    for _ in 0..40 {
        wait(&mut s, &mut c, 0.5);
        step(&mut s, &mut c, &[Action::Confirm]);
    }
    assert_eq!(durability(s.state(), UnitId(1)), 0);
    let after = s.state().clone();
    let buf = info(&mut c, &after, UnitId(1));
    let weapons = text(&buf, 56, 2, 43);
    assert!(weapons.ends_with("broken"), "{weapons}");
}

#[test]
fn a_broken_weapon_dims_every_art_with_the_reason() {
    let mut c = ctx();
    let state = swordsman(&c, 0);
    let s = lord_on_brigand(&mut c, state);
    let shown = lines(&s);
    assert_eq!(shown[0], ("Attack".to_owned(), true, None));
    for line in &shown[1..] {
        assert_eq!(
            (line.1, line.2.as_deref()),
            (false, Some("broken")),
            "{line:?}"
        );
    }
}

#[test]
fn lines_are_attack_then_arts_then_actives_and_only_what_fits() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        assert!(units[0].learn_skill(&SkillId::new("keen_edge"), &c.content.skills));
    });
    let s = lord_on_brigand(&mut c, state);
    let names: Vec<String> = lines(&s).into_iter().map(|l| l.0).collect();
    // No bow or axe arts, and not the lord's Inspire (not a combat active).
    assert_eq!(names, ["Attack", "Flowing Cut", "Guard Break", "Keen Edge"]);
    let labels: Vec<&str> = targeting(&s)
        .list
        .items()
        .iter()
        .map(|i| i.label.trim_end())
        .collect();
    assert_eq!(
        labels,
        [
            "Attack",
            "Flowing Cut     −2 dur  E",
            "Guard Break     −4 dur  D",
            "Keen Edge       −3 dur  active",
        ]
    );
    // The archer: bow arts, and Vault as its active.
    let state = battle(&c, |units| {
        assert!(units[2].learn_skill(&SkillId::new("vault"), &c.content.skills));
    });
    let s = archer_on_brigand(&mut c, state);
    let names: Vec<String> = lines(&s).into_iter().map(|l| l.0).collect();
    assert_eq!(names, ["Attack", "Close Shot", "Pinning Shot", "Vault"]);
}

#[test]
fn a_chosen_active_is_used_and_paid_for() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        assert!(units[0].learn_skill(&SkillId::new("keen_edge"), &c.content.skills));
    });
    let mut s = lord_on_brigand(&mut c, state);
    choose(&mut s, &mut c, "Keen Edge");
    let buf = render(&s, &c);
    assert_eq!(text(&buf, LEFT_X, SKILL_ROW, 26), "Keen Edge (20 → 17)");
    step(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(durability(s.state(), UnitId(1)), 17);
}

#[test]
fn the_chosen_art_stays_across_targets_that_allow_it() {
    let mut c = ctx();
    let state = swordsman(&c, 20);
    let mut s = lord_on_brigand(&mut c, state);
    choose(&mut s, &mut c, "Guard Break");
    // To the raider: still Guard Break, the forecast against the raider.
    step(&mut s, &mut c, &[Action::CursorRight]);
    let t = targeting(&s);
    assert_eq!(t.target(), UnitId(6));
    assert_eq!(t.technique(), Technique::Art(ArtId::new("guard_break")));
    let art = Technique::Art(ArtId::new("guard_break"));
    let expected = s
        .state()
        .preview_attack(
            UnitId(1),
            p(7, 2),
            &art.action(UnitId(6), &Equipped::Weapon(0)),
        )
        .unwrap();
    assert_eq!(t.preview, expected);
}

#[test]
fn the_list_hides_and_up_down_pick_targets_without_one() {
    let mut c = ctx();
    let state = swordsman(&c, 20);
    let mut s = lord_on_brigand(&mut c, state);
    let Mode::Targeting(t) = &mut s.mode else {
        unreachable!()
    };
    t.choices.truncate(1);
    t.list = super::art_list::art_menu(&s.state, &t.choices, 0);
    assert!(!s.help(&c).contains("art"), "{}", s.help(&c));
    assert!(!screen_text(&render(&s, &c)).contains("Iron Sword 20/20"));
    step(&mut s, &mut c, &[Action::CursorDown]);
    assert_eq!(targeting(&s).target(), UnitId(6));
}

/// The lord (Swordsman) with Guard Break chosen: the list beside the panel,
/// the art's line and `no counter`.
#[test]
fn arts_list_and_forecast_with_an_art_snapshot() {
    let mut c = ctx();
    let state = swordsman(&c, 20);
    let mut s = lord_on_brigand(&mut c, state);
    choose(&mut s, &mut c, "Guard Break");
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

/// The archer with Pinning Shot: a note-only art.
#[test]
fn forecast_with_pinning_shot_snapshot() {
    let mut c = ctx();
    let state = battle(&c, |_| {});
    let mut s = archer_on_brigand(&mut c, state);
    choose(&mut s, &mut c, "Pinning Shot");
    let buf = render(&s, &c);
    let all = screen_text(&buf);
    assert!(all.contains("Pinning Shot (20 → 17)"), "{all}");
    assert!(all.contains("pins: Mov −3"), "{all}");
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

/// The lord's iron sword broken: `broken` in the weapon list, `(broken)` in
/// the forecast, and every art dimmed.
#[test]
fn broken_weapon_lines_snapshot() {
    let mut c = ctx();
    let mut s = BattleScreen::new(swordsman(&c, 0));
    step(
        &mut s,
        &mut c,
        &[Action::Confirm, Action::CursorRight, Action::Confirm],
    );
    wait(&mut s, &mut c, 0.5);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::WeaponMenu { menu, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let (text, color) = menu.items()[0].suffix.clone().unwrap();
    assert_eq!((text.trim(), color), ("broken", UiColor::HpLow));
    let list = render(&s, &c).to_snapshot(&c.palette);
    step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
    let forecast = render(&s, &c).to_snapshot(&c.palette);
    assert_snapshot!(format!("{list}\n=== forecast ===\n{forecast}"));
}

#[test]
fn a_pin_shows_on_the_info_screen_with_when_it_ends() {
    let mut c = ctx();
    let plain = battle(&c, |_| {});
    // The archer's Pinning Shot hits the brigand (seed 0), which lives.
    let mut pinned = plain.clone();
    let art = Technique::Art(ArtId::new("pinning_shot"));
    let cmd = Command::Act {
        unit: UnitId(3),
        dest: p(8, 4),
        action: art.action(UnitId(4), &Equipped::Weapon(0)),
    };
    pinned.apply(&cmd).unwrap();
    let brigand = pinned.unit(UnitId(4)).unwrap();
    let [effect] = brigand.effects.as_slice() else {
        panic!("{:?}", brigand.effects);
    };
    assert_eq!(effect.source, EffectSource::Art(ArtId::new("pinning_shot")));
    // Until the end of the Enemy phase: the start of the next one.
    let until = format!("until {:?} phase", effect.until);
    let buf = info(&mut c, &pinned, UnitId(4));
    let all = screen_text(&buf);
    for part in ["Effects", "Pinning Shot", "Mov -3", until.as_str()] {
        assert!(all.contains(part), "{part}: {all}");
    }
    assert!(all.contains("Mov 2"), "{all}");
    // On the map, the brigand at (8, 2) is shown under an effect (the glyph
    // skin puts the effect colour behind its label).
    let under_effect = |state: &BattleState, c: &Ctx| {
        let scene = BattleScreen::new(state.clone()).scene(c);
        let brigand = scene.unit_at(p(8, 2)).unwrap();
        assert_eq!((brigand.id, brigand.label.as_str()), (UnitId(4), "Br"));
        brigand.has_effect()
    };
    assert!(under_effect(&pinned, &c));
    assert!(!under_effect(&plain, &c));
}

#[test]
fn a_boss_or_green_units_art_names_the_playback_and_the_players_doesnt() {
    let c = ctx();
    let state = swordsman(&c, 20);
    let used = |unit| Event::ArtUsed {
        unit,
        art: ArtId::new("guard_break"),
        weapon: trpg_core::ItemId::new("iron_sword"),
        durability_before: 20,
        durability_after: 16,
    };
    let units = state.units().to_vec();
    assert_eq!(playback_banner(&state, &[used(UnitId(1))], &units), None);
    assert_eq!(
        playback_banner(&state, &[used(UnitId(4))], &units).as_deref(),
        Some("Guard Break")
    );
    let mut green = units.clone();
    green[3].faction = Faction::Ally;
    let skill = Event::SkillUsed {
        unit: UnitId(4),
        skill: SkillId::new("keen_edge"),
    };
    assert_eq!(
        playback_banner(&state, &[skill], &green).as_deref(),
        Some("Keen Edge")
    );
    let own = Event::SkillUsed {
        unit: UnitId(1),
        skill: SkillId::new("keen_edge"),
    };
    assert_eq!(playback_banner(&state, &[own], &units), None);
    // Drawn in the box's top border.
    let mut after = state.clone();
    let events = after
        .apply(&trpg_core::Command::Act {
            unit: UnitId(1),
            dest: p(7, 2),
            action: Technique::Attack.action(UnitId(4), &Equipped::Weapon(0)),
        })
        .unwrap();
    let pb = Playback::new(&events, state.units(), after.fallen(), TIMINGS)
        .unwrap()
        .with_banner(Some("Guard Break".to_owned()));
    let mut buf = GlyphBuffer::new(
        CONSOLE_W,
        CONSOLE_H,
        Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)),
    );
    draw_box(&mut buf, &c.palette, &pb);
    let top = text(&buf, BOX.x, BOX.y, BOX.w);
    assert!(top.contains(" Guard Break "), "{top}");
    // Centred: " Guard Break " is 13 cells, (44 - 13) / 2 = 15 in.
    let x = BOX.x + 16;
    assert_eq!(text(&buf, x, BOX.y, 11), "Guard Break");
    assert_eq!(
        buf.get(x, BOX.y).unwrap().fg,
        c.palette.get(UiColor::TextHighlight)
    );
}

/// The archer targeting from where it stands, cycled with Right until
/// `target` is under the cursor.
fn archer_on(c: &mut Ctx, state: BattleState, target: UnitId) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    s.cursor.jump(p(8, 4));
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::Confirm, Action::Confirm],
    );
    for _ in 0..4 {
        if targeting(&s).target() == target {
            return s;
        }
        step(&mut s, c, &[Action::CursorRight]);
    }
    panic!("{target:?} isn't a target: {:?}", targeting(&s).targets);
}

#[test]
fn close_shot_reaches_an_adjacent_enemy_that_a_plain_shot_cant() {
    let mut c = ctx();
    // The brigand (unit 5) at (7, 4), beside the archer at (8, 4).
    let state = battle(&c, |_| {});
    assert_eq!(state.unit(UnitId(5)).unwrap().pos, p(7, 4));
    let mut s = archer_on(&mut c, state, UnitId(5));
    let shown = lines(&s);
    assert_eq!(
        shown[0],
        ("Attack".to_owned(), false, Some("out of range".to_owned()))
    );
    // Pinning Shot keeps the bow's range: not listed here.
    let names: Vec<&str> = shown.iter().map(|l| l.0.as_str()).collect();
    assert_eq!(names, ["Attack", "Close Shot"]);
    // Close Shot is chosen for you; Up/Down can't pick the plain shot.
    assert_eq!(
        targeting(&s).technique(),
        Technique::Art(ArtId::new("close_shot"))
    );
    step(&mut s, &mut c, &[Action::CursorDown]);
    assert_eq!(targeting(&s).technique().name(s.state()), "Close Shot");
    // Back to the brigand two tiles up: Close Shot still works there, so
    // it stays chosen, and the forecast says what it costs.
    step(&mut s, &mut c, &[Action::CursorLeft]);
    assert_eq!(targeting(&s).target(), UnitId(4));
    let buf = render(&s, &c);
    assert_eq!(text(&buf, LEFT_X, SKILL_ROW, 26), "Close Shot (20 → 18)");
    // Shoot the adjacent one: paid 2.
    step(&mut s, &mut c, &[Action::CursorRight, Action::Confirm]);
    assert_eq!(durability(s.state(), UnitId(3)), 18);
}

/// The archer knowing Long Shot, the brigand (unit 4) at `brigand`.
fn long_shot(c: &Ctx, brigand: Pos) -> BattleState {
    battle(c, |units| {
        units[3].pos = brigand;
        assert!(units[2].learn_skill(&SkillId::new("long_shot"), &c.content.skills));
    })
}

#[test]
fn long_shot_reaches_a_target_just_beyond_the_bows_range() {
    let mut c = ctx();
    // Three tiles up: past the bow's two, within Long Shot's extra reach.
    let state = long_shot(&c, p(8, 1));
    let mut s = archer_on(&mut c, state, UnitId(4));
    let shown = lines(&s);
    assert_eq!(shown[0].2.as_deref(), Some("out of range"));
    let names: Vec<&str> = shown.iter().map(|l| l.0.as_str()).collect();
    assert_eq!(names, ["Attack", "Long Shot"]);
    assert_eq!(targeting(&s).technique().name(s.state()), "Long Shot");
    let before = durability(s.state(), UnitId(3));
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(durability(s.state(), UnitId(3)) < before);
    // Within the bow's range, Long Shot is one more line after the arts.
    let state = long_shot(&c, p(8, 2));
    let s = archer_on(&mut c, state, UnitId(4));
    let names: Vec<String> = lines(&s).into_iter().map(|l| l.0).collect();
    assert_eq!(
        names,
        ["Attack", "Close Shot", "Pinning Shot", "Long Shot", "Vault"]
    );
    // The raider four tiles away (only Long Shot reaches it, +2) came
    // first; Long Shot stays chosen on the brigand, where it works too.
    let t = targeting(&s);
    assert_eq!(t.targets.first(), Some(&UnitId(6)));
    assert_eq!(t.technique(), Technique::Active(SkillId::new("long_shot")));
}

#[test]
fn a_target_beyond_every_line_isnt_offered() {
    let c = ctx();
    // Three tiles up, without Long Shot: nothing reaches.
    let state = battle(&c, |units| units[3].pos = p(8, 1));
    let reach = super::attack::reachable(&state, UnitId(3), p(8, 4), 0);
    assert!(!reach.contains(&UnitId(4)), "{reach:?}");
    let reach = super::attack::reachable(&long_shot(&c, p(8, 1)), UnitId(3), p(8, 4), 0);
    assert!(reach.contains(&UnitId(4)), "{reach:?}");
}
