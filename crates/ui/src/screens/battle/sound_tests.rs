//! Harness tests of the battle sounds (ticket 0424): scripted combats, one
//! per rule of `docs/design/audio.md`'s table, heals, walks and moves of
//! each movement type, and skipping a playback.

use trpg_core::{
    BattlePack, BattleState, ClassId, CombatOutcome, Command, Element, Event, Forecast, ItemId,
    Objective, Pos, Side, SideForecast, StatValue, Strike, UnitAction, UnitId, WeaponKind,
};

use super::BattleScreen;
use super::mode::{Mode, WALK_TILES_PER_S};
use super::playback::{Beat, Playback, TIMINGS};
use super::quick_battle;
use super::sounds::{Attack, sound_for_strike};
use super::testing::{battle_packed, battle_with, skirmish};
use crate::audio::AudioRequest;
use crate::harness::{FRAME_DT, Harness};
use crate::screen::tests::ctx;

/// The sound cues of `requests`, in order.
fn cues(requests: &[AudioRequest]) -> Vec<String> {
    requests
        .iter()
        .filter_map(|r| match r {
            AudioRequest::PlaySound { cue, .. } => Some(cue.clone()),
            _ => None,
        })
        .collect()
}

/// Runs `seconds` of frames of [`FRAME_DT`] and returns each sound cue
/// heard with the time its frame ended.
fn listen(h: &mut Harness, seconds: f32) -> Vec<(f32, String)> {
    let mut heard = Vec::new();
    let mut t = 0.0;
    while t < seconds {
        h.wait(FRAME_DT);
        t += FRAME_DT;
        heard.extend(cues(h.last_frame_audio()).into_iter().map(|c| (t, c)));
    }
    heard
}

fn strike(by: Side, hit: bool, crit: bool, damage: StatValue, after: StatValue) -> Strike {
    Strike {
        by,
        hit,
        crit,
        damage,
        healed: false,
        target_hp_after: after,
    }
}

fn numbers() -> SideForecast {
    SideForecast {
        damage: 7,
        followup_damage: 7,
        hit: 90,
        crit: 3,
        strikes: 1,
        effective: false,
        broken: false,
        affinity: None,
    }
}

/// The skirmish's battle screen playing a combat of the lord (unit 1)
/// against the brigand (unit 4) with `strikes`, the lord striking with
/// `attacks[0]` and the brigand with `attacks[1]`, in the harness; and
/// that playback.
fn scripted(strikes: Vec<Strike>, attacks: [Option<Attack>; 2]) -> (Harness, Playback) {
    let state = skirmish(&ctx(), 20);
    let before = state.units().to_vec();
    let events = [Event::CombatResolved {
        attacker: UnitId(1),
        defender: UnitId(4),
        forecast: Forecast {
            attacker: numbers(),
            defender: Some(numbers()),
        },
        outcome: CombatOutcome {
            strikes,
            attacker_hp: 19,
            defender_hp: 13,
        },
    }];
    let playback = Playback::new(&events, &before, &[], TIMINGS)
        .expect("a combat")
        .with_sounds(&[attacks], false);
    let mut screen = BattleScreen::new(state);
    screen.mode = Mode::Combat(Box::new(playback.clone()));
    (Harness::with_screen(Box::new(screen)), playback)
}

/// Plays `strikes` with `attacks` to the end and checks that exactly
/// `expected` is heard, each cue once, in the frame its beat starts.
fn assert_sounds(strikes: Vec<Strike>, attacks: [Option<Attack>; 2], expected: &[(Beat, &str)]) {
    let (mut h, playback) = scripted(strikes, attacks);
    let heard = listen(&mut h, playback.total() + 0.5);
    let names: Vec<&str> = heard.iter().map(|(_, c)| c.as_str()).collect();
    let wanted: Vec<&str> = expected.iter().map(|(_, c)| *c).collect();
    assert_eq!(names, wanted, "{attacks:?}");
    for ((t, cue), (beat, _)) in heard.iter().zip(expected) {
        let step = playback.steps().iter().find(|s| s.beat == *beat).unwrap();
        // Heard in the frame that reached the beat's start.
        assert!(
            *t >= step.start - 1e-4 && *t - FRAME_DT < step.start,
            "{cue} at {t}, its beat {beat:?} at {}",
            step.start
        );
    }
}

const FLASH: Beat = Beat::Flash { bout: 0, strike: 0 };
const RESULT: Beat = Beat::Result { bout: 0, strike: 0 };

const SWORD: Option<Attack> = Some(Attack::Weapon(WeaponKind::Sword));
const AXE: Option<Attack> = Some(Attack::Weapon(WeaponKind::Axe));

const FIRE: Option<Attack> = Some(Attack::Spell(Element::Fire));
const ICE: Option<Attack> = Some(Attack::Spell(Element::Ice));
const NO_ELEMENT: Option<Attack> = Some(Attack::Spell(Element::None));

#[test]
fn a_miss_whooshes() {
    let miss = vec![strike(Side::Attacker, false, false, 0, 20)];
    assert_sounds(miss, [SWORD, AXE], &[(RESULT, "miss")]);
}

#[test]
fn a_weapon_hit_sounds_by_the_strikers_weapon_kind() {
    let kinds = [
        (WeaponKind::Sword, "hit_sword"),
        (WeaponKind::Spear, "hit_spear"),
        (WeaponKind::Axe, "hit_axe"),
        (WeaponKind::Bow, "hit_bow"),
        (WeaponKind::Gauntlet, "hit_gauntlet"),
    ];
    for (kind, cue) in kinds {
        let hit = vec![strike(Side::Attacker, true, false, 7, 13)];
        let with = Some(Attack::Weapon(kind));
        assert_sounds(hit, [with, AXE], &[(RESULT, cue)]);
    }
}

#[test]
fn a_counter_sounds_by_the_defenders_weapon() {
    let strikes = vec![
        strike(Side::Attacker, true, false, 7, 13),
        strike(Side::Defender, true, false, 5, 14),
    ];
    let second = Beat::Result { bout: 0, strike: 1 };
    let expected = [(RESULT, "hit_sword"), (second, "hit_axe")];
    assert_sounds(strikes, [SWORD, AXE], &expected);
}

#[test]
fn a_weapon_crit_plays_crit_physical_instead_of_the_hit() {
    let crit = vec![strike(Side::Attacker, true, true, 20, 0)];
    assert_sounds(crit, [SWORD, AXE], &[(RESULT, "crit_physical")]);
}

#[test]
fn a_hit_for_no_damage_is_blocked_weapon_or_spell() {
    let block = || vec![strike(Side::Attacker, true, false, 0, 20)];
    assert_sounds(block(), [SWORD, AXE], &[(RESULT, "block")]);
    let expected = [(FLASH, "cast_ice"), (RESULT, "block")];
    assert_sounds(block(), [ICE, AXE], &expected);
}

#[test]
fn a_spell_casts_by_element_then_lands_with_hit_magic() {
    let hit = || vec![strike(Side::Attacker, true, false, 7, 13)];
    let fire = [(FLASH, "cast_fire"), (RESULT, "hit_magic")];
    assert_sounds(hit(), [FIRE, AXE], &fire);
    let ice = [(FLASH, "cast_ice"), (RESULT, "hit_magic")];
    assert_sounds(hit(), [ICE, AXE], &ice);
    // No element: no cast sound.
    assert_sounds(hit(), [NO_ELEMENT, AXE], &[(RESULT, "hit_magic")]);
    // A miss still casts.
    let miss = vec![strike(Side::Attacker, false, false, 0, 20)];
    let expected = [(FLASH, "cast_fire"), (RESULT, "miss")];
    assert_sounds(miss, [FIRE, AXE], &expected);
}

#[test]
fn a_spell_crit_plays_the_elements_crit_instead_of_hit_magic() {
    let crit = || vec![strike(Side::Attacker, true, true, 20, 0)];
    let fire = [(FLASH, "cast_fire"), (RESULT, "crit_fire")];
    assert_sounds(crit(), [FIRE, AXE], &fire);
    let ice = [(FLASH, "cast_ice"), (RESULT, "crit_ice")];
    assert_sounds(crit(), [ICE, AXE], &ice);
    // audio.md's open sub-question: a no-element crit is a plain landing.
    assert_sounds(crit(), [NO_ELEMENT, AXE], &[(RESULT, "hit_magic")]);
}

#[test]
fn an_absorb_strike_sounds_like_a_heal() {
    let absorb = vec![Strike {
        healed: true,
        ..strike(Side::Attacker, true, false, 6, 20)
    }];
    let expected = [(FLASH, "cast_fire"), (RESULT, "heal")];
    assert_sounds(absorb, [FIRE, AXE], &expected);
}

/// The lord's attack on the brigand in the skirmish, through the keys: it
/// walks one tile right, picks Attack, the sword and the brigand, and
/// confirms.
fn attack(h: &mut Harness) {
    h.keys("f Right f").wait(0.5);
    h.keys("f f Right f");
}

#[test]
fn a_real_combat_sounds_each_strike_by_its_outcome() {
    let c = ctx();
    let state = skirmish(&c, 20);
    let mut after = state.clone();
    let cmd = Command::Act {
        unit: UnitId(1),
        dest: Pos::new(7, 2),
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    };
    let events = after.apply(&cmd).unwrap_or_else(|e| panic!("{e}"));
    let brigand = state.unit(UnitId(4)).unwrap();
    let axe = brigand
        .loadout
        .equipped_weapon()
        .and_then(|w| state.items().weapon(&w.def))
        .map(|w| Attack::Weapon(w.kind));
    let mut expected = vec!["step_foot".to_owned()];
    for e in &events {
        if let Event::CombatResolved { outcome, .. } = e {
            for s in &outcome.strikes {
                let with = if s.by == Side::Attacker { SWORD } else { axe };
                expected.extend(sound_for_strike(s, with).map(str::to_owned));
            }
        }
    }
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    attack(&mut h);
    h.wait(10.0);
    assert_eq!(cues(&h.audio_requests()), expected);
}

#[test]
fn skipping_the_playback_plays_none_of_its_sounds_left() {
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(skirmish(&ctx(), 20))));
    attack(&mut h);
    let started = h.audio_requests().len();
    // Cancel before the first strike's result.
    h.keys("d");
    h.wait(10.0);
    assert_eq!(cues(&h.audio_requests()[started..]), Vec::<String>::new());
}

/// The Quick Battle's units, only the lord and the knight (unit 2), the
/// lord at `lord`, the knight at `knight` missing `hurt` HP, and potions.
fn pair(lord: Pos, knight: Pos, hurt: StatValue) -> BattleState {
    let c = ctx();
    let quick = quick_battle(&c.content).unwrap();
    let mut units = quick.units()[..2].to_vec();
    units[0].pos = lord;
    units[1].pos = knight;
    units[1].hp -= hurt;
    // A brigand far off, so the battle isn't won.
    let mut brigand = quick.units()[3].clone();
    brigand.pos = Pos::new(13, 0);
    units.push(brigand);
    let pack = BattlePack::bring(vec![ItemId::new("potion")], 6).expect("fits");
    let rout = Objective::Rout { turn_limit: None };
    battle_packed(&c, quick.map().clone(), units, rout, 0, pack)
}

/// `screen` in the harness after one frame.
fn one_frame(screen: BattleScreen) -> Harness {
    let mut h = Harness::with_screen(Box::new(screen));
    h.wait(FRAME_DT);
    h
}

#[test]
fn a_unit_healed_plays_heal_and_healing_nothing_plays_nothing() {
    let use_potion = Command::Act {
        unit: UnitId(1),
        dest: Pos::new(3, 5),
        action: UnitAction::UseItem {
            pack_index: 0,
            target: UnitId(2),
        },
    };
    let mut screen = BattleScreen::new(pair(Pos::new(3, 5), Pos::new(4, 5), 5));
    screen.apply(&use_potion);
    let h = one_frame(screen);
    assert_eq!(cues(&h.audio_requests()), ["heal"]);
    // On a unit at full HP (the core allows it): nothing healed, no sound.
    let mut screen = BattleScreen::new(pair(Pos::new(3, 5), Pos::new(4, 5), 0));
    screen.apply(&use_potion);
    let mut h = one_frame(screen);
    h.wait(1.0);
    assert_eq!(cues(&h.audio_requests()), Vec::<String>::new());
}

#[test]
fn a_foot_unit_walking_five_tiles_takes_five_steps_as_it_enters_each() {
    // The lord at (3, 6) walks right along the plain to (8, 6).
    let state = pair(Pos::new(3, 6), Pos::new(4, 5), 0);
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    h.keys("f Right Right Right Right Right");
    assert_eq!(cues(&h.audio_requests()), Vec::<String>::new());
    // The confirm's press frame starts the walk; its release frame is the
    // walk's second (1 / 30 s in: no tile entered yet).
    h.keys("f");
    assert_eq!(cues(&h.audio_requests()), Vec::<String>::new());
    let heard: Vec<(f32, String)> = listen(&mut h, 1.0)
        .into_iter()
        .map(|(t, c)| (t + 2.0 * FRAME_DT, c))
        .collect();
    let names: Vec<&str> = heard.iter().map(|(_, c)| c.as_str()).collect();
    assert_eq!(names, ["step_foot"; 5]);
    // Tile k is heard in the frame that reaches k / speed seconds.
    for (k, (t, _)) in heard.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let at = (k + 1) as f32 / WALK_TILES_PER_S;
        assert!(*t >= at - 1e-4 && *t - FRAME_DT < at, "step {k} at {t}");
    }
    // Waiting where it stands afterwards adds no steps.
    h.keys("Up Up f");
    h.wait(1.0);
    assert_eq!(cues(&h.audio_requests()).len(), 5);
}

#[test]
fn confirm_skipping_a_walk_skips_its_steps_left() {
    let state = pair(Pos::new(3, 6), Pos::new(4, 5), 0);
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    h.keys("f Right Right Right Right Right f f");
    h.wait(1.0);
    // Two frames of walk (1 / 30 s) before the skip: no tile entered yet,
    // and none after.
    assert_eq!(cues(&h.audio_requests()), Vec::<String>::new());
}

#[test]
fn moves_without_a_walk_step_by_movement_type() {
    let classes = [
        ("swordsman", Some("step_foot")),
        ("guard", Some("step_armored")),
        ("rider", Some("step_mounted")),
        ("flier", None),
    ];
    for (class, cue) in classes {
        let mut state = pair(Pos::new(3, 6), Pos::new(4, 5), 0);
        let mut units = state.units().to_vec();
        units[0].class = ClassId(class.into());
        let c = ctx();
        state = battle_with(&c, state.map().clone(), units, state.objective());
        let mut screen = BattleScreen::new(state);
        // A move sent without the walk being shown (as a later enemy
        // phase's): the path's two tiles right.
        screen.apply(&Command::Act {
            unit: UnitId(1),
            dest: Pos::new(5, 6),
            action: UnitAction::Wait,
        });
        let mut h = Harness::with_screen(Box::new(screen));
        let heard = listen(&mut h, 1.0);
        let names: Vec<&str> = heard.iter().map(|(_, c)| c.as_str()).collect();
        let expected: Vec<&str> = cue.into_iter().cycle().take(2).collect();
        assert_eq!(names, expected, "{class}");
        if cue.is_some() {
            // A step per tile, at the walk's pace.
            let gap = heard[1].0 - heard[0].0;
            assert!((gap - 1.0 / WALK_TILES_PER_S).abs() <= FRAME_DT + 1e-4);
        }
    }
}

#[test]
fn a_heal_in_a_combat_command_plays_as_the_outro_starts() {
    let state = skirmish(&ctx(), 20);
    let before = state.units().to_vec();
    let events = [Event::CombatResolved {
        attacker: UnitId(1),
        defender: UnitId(4),
        forecast: Forecast {
            attacker: numbers(),
            defender: None,
        },
        outcome: CombatOutcome {
            strikes: vec![strike(Side::Attacker, true, false, 7, 13)],
            attacker_hp: 19,
            defender_hp: 13,
        },
    }];
    let playback = Playback::new(&events, &before, &[], TIMINGS)
        .unwrap()
        .with_sounds(&[[SWORD, AXE]], true);
    let outro = playback.steps().last().unwrap();
    assert_eq!(outro.beat, Beat::Outro);
    assert_eq!(playback.cues().last(), Some(&(outro.start, "heal")));
    // The screen leaves it to the playback.
    let cues = super::sounds::event_cues(
        &[Event::Healed {
            target: UnitId(1),
            amount: 3,
        }],
        None,
        true,
        &state,
    );
    assert!(cues.is_empty());
}

#[test]
fn a_spell_cast_outside_a_combat_plays_its_cast_sound_at_once() {
    let state = skirmish(&ctx(), 20);
    let spells = state.spells();
    let by = |element| {
        let (id, _) = spells
            .spells
            .iter()
            .find(|(_, s)| s.element == element)
            .unwrap();
        Event::SpellCast {
            unit: UnitId(1),
            spell: id.clone(),
            target: trpg_core::CastTarget::Tile(Pos::new(3, 3)),
        }
    };
    let cues = |e: Event| super::sounds::event_cues(&[e], None, false, &state);
    assert_eq!(cues(by(Element::Fire)), [(0.0, "cast_fire")]);
    assert_eq!(cues(by(Element::Ice)), [(0.0, "cast_ice")]);
    assert_eq!(cues(by(Element::None)), []);
    // In a combat command the strikes cast instead.
    let fire = by(Element::Fire);
    assert!(super::sounds::event_cues(&[fire], None, true, &state).is_empty());
}
