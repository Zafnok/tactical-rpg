//! Which menu sound a key plays on the battle screen (ticket 0425,
//! `docs/design/audio.md`). The move loop's menus are plain data stepped by
//! [`mode::step`](super::mode::step), so the sound is read off what a step
//! changed: nothing changed, nothing plays.

use std::cmp::Ordering;

use super::mode::{Effect, Mode};
use crate::audio::MenuSound;
use crate::input::Action;

/// The sound `action` makes by turning `before` into `after` with `effect`,
/// if any. Opening something sounds as `Select`, whichever key did it
/// (Cancel opens the map menu). Otherwise Confirm sounds as `Select` and
/// Cancel as `Cancel`; any other key sounds as `Select` if it issues a
/// command, `Cancel` if it closes something (e.g. `Info` again), and `Move`
/// if it moves within the same screen (a menu's focus, a target, the info
/// screen's unit). Skipping a walk or a combat, or anything during an AI
/// action, plays nothing.
pub fn step_sound(
    action: Action,
    before: &Mode,
    after: &Mode,
    effect: &Effect,
) -> Option<MenuSound> {
    if matches!(
        before,
        Mode::Moving { .. } | Mode::Combat(_) | Mode::AiAction(_)
    ) {
        return None;
    }
    if before == after && *effect == Effect::None {
        return None;
    }
    let deeper = depth(after).cmp(&depth(before));
    Some(match action {
        _ if deeper == Ordering::Greater => MenuSound::Select,
        Action::Confirm => MenuSound::Select,
        Action::Cancel => MenuSound::Cancel,
        _ if matches!(effect, Effect::Apply(_) | Effect::ApplyStay(..)) => MenuSound::Select,
        _ if deeper == Ordering::Less => MenuSound::Cancel,
        _ => MenuSound::Move,
    })
}

/// How many steps `mode` is from browsing the map: opening something goes
/// deeper, backing out shallower.
fn depth(mode: &Mode) -> u8 {
    match mode {
        Mode::Idle { .. } | Mode::MoveAfter { .. } | Mode::Combat(_) | Mode::AiAction(_) => 0,
        Mode::Selected(_)
        | Mode::Moving { .. }
        | Mode::MapMenu { .. }
        | Mode::EndTurnPrompt { .. }
        | Mode::Info { .. } => 1,
        Mode::ActionMenu { .. }
        | Mode::UnitList { .. }
        | Mode::Objective
        | Mode::RestartPrompt
        | Mode::SuspendPrompt => 2,

        Mode::WeaponMenu { .. }
        | Mode::SkillMenu { .. }
        | Mode::ItemMenu { .. }
        | Mode::EquipMenu { .. }
        | Mode::TalkTarget { .. } => 3,
        Mode::Targeting(_) | Mode::SkillTarget(_) | Mode::ItemTarget(_) => 4,
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Command, Pos, UnitId};

    use super::*;
    use crate::audio::MenuSound::{Cancel, Move, Select};

    const IDLE: Mode = Mode::Idle { threat: None };
    const INFO: Mode = Mode::Info { unit: UnitId(1) };
    const PROMPT: Mode = Mode::EndTurnPrompt { ready: 2 };

    fn sound(action: Action, before: &Mode, after: &Mode, effect: &Effect) -> Option<MenuSound> {
        step_sound(action, before, after, effect)
    }

    #[test]
    fn nothing_changed_nothing_plays() {
        for a in [
            Action::Confirm,
            Action::Cancel,
            Action::CursorDown,
            Action::Info,
        ] {
            assert_eq!(sound(a, &IDLE, &IDLE, &Effect::None), None);
            assert_eq!(sound(a, &INFO, &INFO, &Effect::None), None);
        }
    }

    #[test]
    fn confirm_and_cancel_sound_as_themselves() {
        let at = Effect::Cursor(Pos::new(1, 1));
        // Confirm leaving the unit list for the map, or showing a threat.
        assert_eq!(sound(Action::Confirm, &PROMPT, &IDLE, &at), Some(Select));
        assert_eq!(sound(Action::Confirm, &IDLE, &IDLE, &at), Some(Select));
        assert_eq!(
            sound(Action::Cancel, &PROMPT, &IDLE, &Effect::None),
            Some(Cancel)
        );
        // Hiding a threat area.
        assert_eq!(sound(Action::Cancel, &IDLE, &IDLE, &at), Some(Cancel));
    }

    #[test]
    fn opening_sounds_as_select() {
        // Cancel opening the map menu.
        assert_eq!(
            sound(Action::Cancel, &IDLE, &PROMPT, &Effect::None),
            Some(Select)
        );
        assert_eq!(
            sound(Action::Info, &IDLE, &INFO, &Effect::None),
            Some(Select)
        );
    }

    #[test]
    fn other_keys_open_close_or_move() {
        assert_eq!(
            sound(Action::Info, &IDLE, &INFO, &Effect::None),
            Some(Select)
        );
        assert_eq!(
            sound(Action::Info, &INFO, &IDLE, &Effect::Cursor(Pos::new(1, 1))),
            Some(Cancel)
        );
        let next = Mode::Info { unit: UnitId(2) };
        assert_eq!(
            sound(Action::NextUnit, &INFO, &next, &Effect::None),
            Some(Move)
        );
        assert_eq!(
            sound(Action::CursorDown, &INFO, &next, &Effect::None),
            Some(Move)
        );
        let end = Effect::Apply(Command::EndPhase);
        assert_eq!(sound(Action::EndTurn, &PROMPT, &IDLE, &end), Some(Select));
        assert_eq!(
            sound(Action::EndTurn, &IDLE, &PROMPT, &Effect::None),
            Some(Select)
        );
    }

    #[test]
    fn depths_order_the_screens() {
        assert!(depth(&IDLE) < depth(&INFO));
        assert!(depth(&INFO) < depth(&Mode::Objective));
        // Picking who to talk to is a step past the action menu.
        let state = crate::screens::battle::quick_battle(&crate::screen::tests::ctx().content)
            .unwrap_or_else(|e| panic!("{e}"));
        let sel = super::super::mode::Selection::new(&state, UnitId(1)).unwrap();
        let talk = Mode::TalkTarget {
            sel,
            targets: vec![UnitId(7)],
            index: 0,
        };
        assert_eq!(depth(&talk), depth(&Mode::Objective) + 1);
    }
}
