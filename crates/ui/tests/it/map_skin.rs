//! The battle map under each map skin (ADR-0038, ticket 0433): a skin
//! changes how the map looks and how many tiles fit, never the game.

use insta::assert_snapshot;
use trpg_core::{BattleState, Pos};
use trpg_ui::audio::AudioRequest;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// What one step of a script left: the cursor's tile, the screens, and the
/// battle (if one is on the stack).
#[derive(Debug, PartialEq)]
struct After {
    step: &'static str,
    cursor: Option<Pos>,
    screens: Vec<&'static str>,
    battle: Option<BattleState>,
}

/// One step: keys pressed (each a frame, then released), then seconds of
/// waiting.
type Step = (&'static str, f32);

/// The Quick Battle from the title: the lord walks up to the brigand and
/// attacks it, the fight is rewound, the turn ends, the enemy phase plays,
/// and turn 2 starts.
const SCRIPT: [Step; 16] = [
    ("Down f", 0.0),
    ("f", 0.0),
    ("f Right Right Right Up", 0.0),
    ("f", 0.5),
    ("f", 0.5),
    ("f", 0.5),
    ("f", 30.0),
    ("Left Down", 0.0),
    ("r", 0.5),
    ("f", 0.0),
    ("f", 0.0),
    ("Space", 0.0),
    ("Space", 0.0),
    ("f", 30.0),
    ("f", 1.0),
    ("Down Down", 0.0),
];

/// Plays [`SCRIPT`] with the map painted by the skin `skin`: what each
/// step left, and every sound and music request of the run.
fn play(skin: &str) -> (Vec<After>, Vec<AudioRequest>) {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.with_map_skin(skin);
    let mut after = Vec::new();
    for (keys, seconds) in SCRIPT {
        h.keys(keys);
        if seconds > 0.0 {
            h.wait(seconds);
        }
        let battle = h.battle();
        after.push(After {
            step: keys,
            cursor: battle.map(|b| b.cursor().pos),
            screens: h.screens(),
            battle: battle.map(|b| b.state().clone()),
        });
    }
    (after, h.audio_requests())
}

/// One long scripted Quick Battle, played under the glyph skin and under
/// the sprite skin, ends the same way: the same battle, the cursor on the
/// same tile and the same screens after every step, the same sounds.
#[test]
fn the_skin_never_changes_the_game() {
    let (glyph, glyph_audio) = play("glyph");
    let (sprite, sprite_audio) = play("sprite");
    // The script did what it says: the lord moved and fought, the fight
    // was rewound, and the enemy phase came and went.
    let state = |i: usize| glyph[i].battle.clone().unwrap();
    let lord = |s: &BattleState| s.units()[0].pos;
    assert_eq!(glyph[0].screens, ["title", "battle"]);
    assert_eq!(lord(&state(6)), Pos::new(6, 4));
    assert_ne!(state(6), state(1));
    assert_eq!(lord(&state(10)), Pos::new(3, 5), "rewound");
    assert!(state(15).turn() >= 2, "turn {}", state(15).turn());
    let sounds = glyph_audio
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlaySound { .. }));
    assert!(sounds.count() > 10);
    // Step by step, the same.
    for (g, s) in glyph.iter().zip(&sprite) {
        assert_eq!(g, s, "after {:?}", g.step);
    }
    assert_eq!(glyph.len(), sprite.len());
    assert_eq!(glyph_audio, sprite_audio);
}

/// The Quick Battle at its start under the sprite skin: the map as tile
/// and unit sprites from the test tileset (23 × 20 tiles of 24 px), the
/// panels and help as glyphs.
#[test]
fn quick_battle_under_the_sprite_skin() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.with_map_skin("sprite");
    h.keys("Down f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert!(
        h.map_text().starts_with("origin (-4,-6) size 23x20\n"),
        "{}",
        h.map_text()
    );
    assert_snapshot!(h.snapshot());
}

/// The same with the lord selected: its move and attack ranges tinting
/// the tiles, the path and the cursor.
#[test]
fn quick_battle_under_the_sprite_skin_with_a_unit_selected() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.with_map_skin("sprite");
    h.keys("Down f f f Right Right");
    assert_snapshot!(h.snapshot());
}
