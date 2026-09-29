//! Checks a battle's dialogue triggers ([`Trigger`], ticket 0705) against
//! the battle and the dialogue: the battle-file loader (0801) runs it on
//! every battle, and the debug Quick Battle on its own.

use trpg_core::{BattleMap, CharacterId, Trigger, TriggerWhen, Unit, Who};

use crate::dialogue::DialogueTable;
use crate::error::ContentError;

/// Every problem with `triggers` of the battle `battle` (a file name, for
/// the errors), one error each: a scene that isn't in `dialogue`, a
/// character that isn't one of `units` (the battle's units, those that
/// arrive later included), an area that is empty or not all on `map`, a
/// talk of a character to itself and a line against its own speaker.
pub fn check_triggers(
    battle: &str,
    triggers: &[Trigger],
    units: &[Unit],
    map: &BattleMap,
    dialogue: &DialogueTable,
) -> Vec<ContentError> {
    let mut errors = Vec::new();
    for (i, t) in triggers.iter().enumerate() {
        let mut error = |message: String| {
            errors.push(ContentError::new(battle, format!("trigger {i}: {message}")));
        };
        if dialogue.get(&t.scene).is_none() {
            error(format!("no dialogue scene \"{}\"", t.scene));
        }
        for c in characters(&t.when) {
            if !units.iter().any(|u| u.character.as_ref() == Some(c)) {
                error(format!("no unit of character \"{}\" in the battle", c.0));
            }
        }
        match &t.when {
            TriggerWhen::UnitEntersArea { area, .. } => {
                let (w, h) = (map.tiles.width(), map.tiles.height());
                let inside = area.w > 0
                    && area.h > 0
                    && area.x >= 0
                    && area.y >= 0
                    && area.x.saturating_add(area.w) <= i32::from(w)
                    && area.y.saturating_add(area.h) <= i32::from(h);
                if !inside {
                    error(format!(
                        "area ({}, {}) {}×{} isn't on the {w}×{h} map",
                        area.x, area.y, area.w, area.h
                    ));
                }
            }
            TriggerWhen::Talk { a, b, .. } if a == b => {
                error(format!("\"{}\" talks to itself", a.0));
            }
            TriggerWhen::CombatStart {
                unit,
                against: Some(against),
                ..
            } if unit == against => {
                error(format!("\"{}\" has a line against itself", unit.0));
            }
            _ => {}
        }
    }
    errors
}

/// The characters a trigger names.
fn characters(when: &TriggerWhen) -> Vec<&CharacterId> {
    match when {
        TriggerWhen::TurnStart { .. } => vec![],
        TriggerWhen::UnitEntersArea { who, .. } => match who {
            Who::Character(c) => vec![c],
            Who::Faction(_) => vec![],
        },
        TriggerWhen::CombatStart { unit, against, .. } => {
            std::iter::once(unit).chain(against).collect()
        }
        TriggerWhen::UnitFell { unit, .. } => vec![unit],
        TriggerWhen::Talk { a, b, .. } => vec![a, b],
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Faction, GameMode, Grid, Phase, Pos, TerrainId, TileRect, UnitId};

    use super::*;
    use crate::dialogue::Scene;

    fn c(id: &str) -> CharacterId {
        CharacterId(id.into())
    }

    /// Units of the placeholder knight, as characters `ana` and `rook`, and
    /// one with no character.
    fn units() -> Vec<Unit> {
        let content = crate::load_embedded().unwrap_or_else(|e| panic!("{e}"));
        let def = &content.characters.characters[&c("test_knight")];
        [Some("ana"), Some("rook"), None]
            .into_iter()
            .zip(1..)
            .map(|(character, id)| {
                let at = Pos::new(0, 0);
                let u = Unit::from_character(UnitId(id), def, &content.classes, Faction::Enemy, at)
                    .unwrap_or_else(|e| panic!("{e}"));
                Unit {
                    character: character.map(c),
                    ..u
                }
            })
            .collect()
    }

    fn dialogue(ids: &[&str]) -> DialogueTable {
        DialogueTable {
            scenes: ids
                .iter()
                .map(|id| {
                    let scene = Scene {
                        id: (*id).into(),
                        steps: vec![],
                    };
                    ((*id).into(), scene)
                })
                .collect(),
        }
    }

    fn trigger(when: TriggerWhen, scene: &str) -> Trigger {
        Trigger {
            when,
            scene: scene.into(),
            once: true,
        }
    }

    fn check(triggers: &[Trigger]) -> Vec<String> {
        let map = BattleMap::new("Test", Grid::filled(6, 4, TerrainId(0)));
        check_triggers("b.ron", triggers, &units(), &map, &dialogue(&["hi", "bye"]))
            .into_iter()
            .map(|e| e.to_string())
            .collect()
    }

    #[test]
    fn good_triggers_pass() {
        let area = TileRect {
            x: 4,
            y: 2,
            w: 2,
            h: 2,
        };
        let good = [
            trigger(
                TriggerWhen::TurnStart {
                    turn: 1,
                    phase: Phase::Player,
                },
                "hi",
            ),
            trigger(
                TriggerWhen::UnitEntersArea {
                    who: Who::Character(c("ana")),
                    area,
                },
                "hi",
            ),
            trigger(
                TriggerWhen::UnitEntersArea {
                    who: Who::Faction(Faction::Player),
                    area,
                },
                "hi",
            ),
            trigger(
                TriggerWhen::CombatStart {
                    unit: c("rook"),
                    against: Some(c("ana")),
                    per_opponent: false,
                },
                "bye",
            ),
            trigger(
                TriggerWhen::UnitFell {
                    unit: c("rook"),
                    mode: Some(GameMode::Casual),
                },
                "bye",
            ),
            trigger(
                TriggerWhen::Talk {
                    a: c("ana"),
                    b: c("rook"),
                    recruit: true,
                },
                "hi",
            ),
        ];
        assert_eq!(check(&good), Vec::<String>::new());
    }

    #[test]
    fn every_problem_is_reported() {
        let area = |x, y, w, h| TileRect { x, y, w, h };
        let enters = |a| TriggerWhen::UnitEntersArea {
            who: Who::Faction(Faction::Enemy),
            area: a,
        };
        let bad = [
            trigger(
                TriggerWhen::UnitFell {
                    unit: c("ghost"),
                    mode: None,
                },
                "missing",
            ),
            trigger(
                TriggerWhen::CombatStart {
                    unit: c("ana"),
                    against: Some(c("nobody")),
                    per_opponent: true,
                },
                "hi",
            ),
            trigger(enters(area(5, 0, 2, 1)), "hi"),
            trigger(enters(area(0, 0, 0, 1)), "hi"),
            trigger(enters(area(-1, 0, 1, 1)), "hi"),
            trigger(
                TriggerWhen::Talk {
                    a: c("ana"),
                    b: c("ana"),
                    recruit: false,
                },
                "hi",
            ),
            trigger(
                TriggerWhen::CombatStart {
                    unit: c("rook"),
                    against: Some(c("rook")),
                    per_opponent: false,
                },
                "hi",
            ),
            trigger(
                TriggerWhen::UnitEntersArea {
                    who: Who::Character(c("zed")),
                    area: area(0, 0, 1, 1),
                },
                "hi",
            ),
        ];
        assert_eq!(
            check(&bad),
            [
                "b.ron: trigger 0: no dialogue scene \"missing\"",
                "b.ron: trigger 0: no unit of character \"ghost\" in the battle",
                "b.ron: trigger 1: no unit of character \"nobody\" in the battle",
                "b.ron: trigger 2: area (5, 0) 2×1 isn't on the 6×4 map",
                "b.ron: trigger 3: area (0, 0) 0×1 isn't on the 6×4 map",
                "b.ron: trigger 4: area (-1, 0) 1×1 isn't on the 6×4 map",
                "b.ron: trigger 5: \"ana\" talks to itself",
                "b.ron: trigger 6: \"rook\" has a line against itself",
                "b.ron: trigger 7: no unit of character \"zed\" in the battle",
            ]
        );
    }
}
