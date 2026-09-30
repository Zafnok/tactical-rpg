//! Tests of the player's key bindings: slots, moves, reserved keys, and the
//! saved config's repair on load.

use proptest::prelude::*;

use super::*;
use crate::input::Key;
use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Debug, Info};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

fn def() -> KeymapDef {
    KeymapDef::load().unwrap_or_else(|e| panic!("{e:?}"))
}

fn right() -> LayoutBindings {
    LayoutBindings::defaults(&def(), Layout::RightHanded)
}

/// Slots as names, `-` for empty.
fn names(slots: Slots) -> [String; SLOTS] {
    slots.map(|c| c.map_or_else(|| "-".to_owned(), |c| c.to_string()))
}

#[test]
fn defaults_fill_the_first_slots_in_file_order() {
    let b = right();
    assert_eq!(names(b.slots(Confirm)), ["f", "-", "-"]);
    assert_eq!(names(b.slots(Cancel)), ["d", "-", "-"]);
    assert_eq!(names(b.slots(Action::Menu)), ["-", "-", "-"]);
    // Debug has no slots, but keeps its key.
    assert_eq!(names(b.slots(Debug)), ["-", "-", "-"]);
    assert_eq!(
        b.keymap(RepeatDef::default()).action(chord("F2")),
        Some(Debug)
    );
    let left = LayoutBindings::defaults(&def(), Layout::LeftHanded);
    assert_eq!(names(left.slots(Confirm)), ["j", "-", "-"]);
    assert!(b.unmapped_required().is_empty());
    assert!(left.unmapped_required().is_empty());
}

#[test]
fn default_bindings_give_the_default_keymap() {
    let def = def();
    for layout in Layout::ALL {
        assert_eq!(
            LayoutBindings::defaults(&def, layout).keymap(def.repeat),
            Keymap::for_layout(&def, layout),
            "{layout}"
        );
    }
}

#[test]
fn a_hand_built_definition_is_repaired_into_the_invariant() {
    let mut def = KeymapDef::default();
    let keys = [
        (Confirm, vec![chord("f"), chord("Escape"), chord("g")]),
        (Info, vec![chord("f"), chord("a"), chord("b"), chord("c")]),
    ];
    def.layouts.insert(Layout::LeftHanded, keys.into());
    let b = LayoutBindings::defaults(&def, Layout::LeftHanded);
    // `f` moved to Info (listed later); Esc skipped; a 4th chord dropped.
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "g"]);
    assert_eq!(names(b.slots(Info)), ["f", "a", "b"]);
}

#[test]
fn binding_a_key_another_action_has_moves_it() {
    let mut b = right();
    // Info's only key `e` goes to Confirm's second slot.
    assert_eq!(b.bind(Confirm, 1, chord("e")), Ok(Some((Info, 0))));
    assert_eq!(names(b.slots(Confirm)), ["f", "e", "-"]);
    assert!(b.is_unmapped(Info));
    assert!(!b.is_unmapped(Confirm));
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("e")), Some(Confirm));
    assert_eq!(km.chords_for(Info), vec![]);
    assert_eq!(km.chords_for(Confirm), vec![chord("f"), chord("e")]);
}

#[test]
fn binding_within_an_action_moves_between_slots() {
    let mut b = right();
    assert_eq!(b.bind(Confirm, 2, chord("f")), Ok(Some((Confirm, 0))));
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "f"]);
    // Binding it where it already is changes nothing.
    assert_eq!(b.bind(Confirm, 2, chord("f")), Ok(None));
    assert_eq!(names(b.slots(Confirm)), ["-", "-", "f"]);
}

#[test]
fn binding_over_a_key_replaces_it() {
    let mut b = right();
    assert_eq!(b.bind(Confirm, 0, chord("Shift+g")), Ok(None));
    assert_eq!(names(b.slots(Confirm)), ["Shift+g", "-", "-"]);
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("f")), None);
    assert_eq!(km.action(chord("g")), None);
    assert_eq!(km.action(chord("Shift+g")), Some(Confirm));
}

#[test]
fn esc_delete_and_the_debug_key_are_reserved() {
    let mut b = right();
    for s in ["Escape", "Delete", "Shift+Escape", "Shift+Delete", "F2"] {
        let c = chord(s);
        assert!(b.is_reserved(c), "{s}");
        assert_eq!(b.bind(Info, 0, c), Err(BindError::Reserved(c)), "{s}");
    }
    assert!(!b.is_reserved(chord("Shift+F2")));
    assert_eq!(b, right(), "a refused bind changes nothing");
    assert_eq!(
        BindError::Reserved(chord("Escape")).to_string(),
        "Escape is a fixed key and can't be bound"
    );
}

#[test]
fn without_debug_tools_the_debug_key_can_be_bound_and_wins() {
    let mut b = LayoutBindings::from_def(&def(), Layout::RightHanded, false);
    assert!(!b.is_reserved(chord("F2")));
    assert!(b.is_reserved(chord("Escape")));
    assert_eq!(b.bind(Info, 1, chord("F2")), Ok(None));
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("F2")), Some(Info));
    assert_eq!(km.chords_for(Debug), vec![]);
}

#[test]
fn debug_and_bad_slots_are_refused() {
    let mut b = right();
    assert_eq!(
        b.bind(Debug, 0, chord("g")),
        Err(BindError::NotRebindable(Debug))
    );
    assert_eq!(
        b.bind(Info, SLOTS, chord("g")),
        Err(BindError::NoSuchSlot(SLOTS))
    );
    assert_eq!(b, right());
    assert_eq!(
        BindError::NotRebindable(Debug).to_string(),
        "Debug can't be rebound"
    );
    assert_eq!(
        BindError::NoSuchSlot(3).to_string(),
        "there is no key slot 3"
    );
}

#[test]
fn clear_empties_a_slot() {
    let mut b = right();
    b.bind(Info, 2, chord("g")).ok();
    b.clear(Info, 0);
    assert_eq!(names(b.slots(Info)), ["-", "-", "g"]);
    b.clear(Info, 2);
    assert!(b.is_unmapped(Info));
    // Slots and actions that don't exist: nothing happens.
    b.clear(Info, SLOTS);
    b.clear(Debug, 0);
    assert_eq!(
        b.keymap(RepeatDef::default()).action(chord("F2")),
        Some(Debug)
    );
}

#[test]
fn unmapped_required_lists_exactly_the_required_actions_with_no_key() {
    let mut b = right();
    for action in Action::ALL {
        for i in 0..SLOTS {
            b.clear(action, i);
        }
    }
    assert_eq!(
        b.unmapped_required(),
        [
            CursorLeft,
            CursorDown,
            CursorUp,
            CursorRight,
            Confirm,
            Cancel,
            Action::EndTurn
        ]
    );
    b.bind(Cancel, 2, chord("x")).ok();
    b.bind(CursorUp, 0, chord("Up")).ok();
    assert_eq!(
        b.unmapped_required(),
        [
            CursorLeft,
            CursorDown,
            CursorRight,
            Confirm,
            Action::EndTurn
        ]
    );
    // Losing an optional action's key doesn't count.
    let mut b = right();
    b.clear(Info, 0);
    assert!(b.unmapped_required().is_empty());
    // Moving Confirm's only key away does.
    b.bind(Info, 0, chord("f")).ok();
    assert_eq!(b.unmapped_required(), [Confirm]);
}

#[test]
fn the_keymap_has_the_fixed_keys() {
    let mut b = right();
    for i in 0..SLOTS {
        b.clear(Cancel, i);
    }
    let km = b.keymap(RepeatDef::default());
    assert_eq!(km.action(chord("Escape")), Some(Cancel));
    assert_eq!(km.action(chord("Delete")), None);
    assert_eq!(km.chords_for(Cancel), vec![]);
    assert_eq!(Keymap::fixed_chords_for(Cancel), vec![chord("Escape")]);
    assert_eq!(Keymap::fixed_chords_for(Confirm), vec![]);
}

/// One edit of a property test.
#[derive(Debug, Clone)]
enum Op {
    Bind(Action, usize, Chord),
    Clear(Action, usize),
}

fn arb_op() -> impl Strategy<Value = Op> {
    let action = prop::sample::select(Action::ALL.to_vec());
    let slot = 0..=SLOTS;
    let key = prop::sample::select(Key::ALL);
    prop_oneof![
        (action.clone(), slot.clone(), key, any::<bool>()).prop_map(|(a, i, key, shift)| Op::Bind(
            a,
            i,
            Chord { key, shift }
        )),
        (action, slot).prop_map(|(a, i)| Op::Clear(a, i)),
    ]
}

proptest! {
    #[test]
    fn no_chord_is_in_two_slots_and_none_is_reserved(
        ops in prop::collection::vec(arb_op(), 0..80),
        layout in prop::sample::select(Layout::ALL.to_vec()),
    ) {
        let mut b = LayoutBindings::defaults(&def(), layout);
        for op in ops {
            match op {
                Op::Bind(a, i, c) => {
                    let before = b.clone();
                    if b.bind(a, i, c).is_ok() {
                        prop_assert_eq!(b.slots(a).get(i).copied().flatten(), Some(c));
                    } else {
                        prop_assert_eq!(&b, &before);
                    }
                }
                Op::Clear(a, i) => b.clear(a, i),
            }
            let all: Vec<Chord> = Action::ALL
                .into_iter()
                .flat_map(|a| b.slots(a).into_iter().flatten())
                .collect();
            let mut unique = all.clone();
            unique.sort_unstable();
            unique.dedup();
            prop_assert_eq!(unique.len(), all.len(), "a chord in two slots: {:?}", all);
            prop_assert!(all.iter().all(|&c| !b.is_reserved(c)));
        }
    }
}

// --- Saved config ----------------------------------------------------------

#[test]
fn player_keys_start_as_the_defaults() {
    let keys = PlayerKeys::default();
    for layout in Layout::ALL {
        assert!(!keys.is_custom(layout));
        assert_eq!(
            keys.bindings(&def(), layout),
            LayoutBindings::defaults(&def(), layout)
        );
    }
}

#[test]
fn each_layout_keeps_its_own_keys() {
    let def = def();
    let mut keys = PlayerKeys::default();
    let mut b = keys.bindings(&def, Layout::RightHanded);
    b.bind(Info, 1, chord("g")).ok();
    keys.set(&def, Layout::RightHanded, b.clone());
    assert!(keys.is_custom(Layout::RightHanded));
    assert!(!keys.is_custom(Layout::LeftHanded));
    assert_eq!(keys.bindings(&def, Layout::RightHanded), b);
    assert_eq!(
        keys.keymap(&def, Layout::LeftHanded),
        Keymap::for_layout(&def, Layout::LeftHanded)
    );
    // Restoring the defaults leaves no entry.
    keys.set(
        &def,
        Layout::RightHanded,
        LayoutBindings::defaults(&def, Layout::RightHanded),
    );
    assert_eq!(keys, PlayerKeys::default());
}

#[test]
fn saved_keys_round_trip() {
    let def = def();
    let mut keys = PlayerKeys::default();
    let mut b = keys.bindings(&def, Layout::LeftHanded);
    b.bind(Confirm, 1, chord("Shift+,")).ok();
    b.bind(Info, 2, chord("\\")).ok();
    b.bind(Info, 0, chord("Kp+")).ok();
    b.clear(Action::Rewind, 0);
    keys.set(&def, Layout::LeftHanded, b);
    let text = keys.to_ron();
    assert!(text.starts_with("PlayerKeys("), "{text}");
    assert!(text.contains("version: 1"), "{text}");
    assert!(
        text.contains("\"Confirm\": [Some(\"j\"), Some(\"Shift+,\"), None]"),
        "{text}"
    );
    assert!(!text.contains("RightHanded"), "{text}");
    assert!(!text.contains("Debug"), "{text}");
    assert_eq!(PlayerKeys::from_ron(&text, &def), (keys, vec![]));
}

#[test]
fn empty_saved_keys_round_trip() {
    let text = PlayerKeys::default().to_ron();
    assert_eq!(
        PlayerKeys::from_ron(&text, &def()),
        (PlayerKeys::default(), vec![])
    );
}

/// A saved config with one right-handed layout whose entries are `actions`
/// (RON map entries).
fn saved(actions: &str) -> String {
    format!("PlayerKeys(version: 1, layouts: {{\"RightHanded\": {{{actions}}}}})")
}

/// Loads `text`: the right-handed bindings and the warnings.
fn load(text: &str) -> (LayoutBindings, Vec<String>) {
    let (keys, warnings) = PlayerKeys::from_ron(text, &def());
    (keys.bindings(&def(), Layout::RightHanded), warnings)
}

#[test]
fn unreadable_saved_keys_give_the_defaults_and_a_warning() {
    for text in [
        "",
        "garbage",
        "PlayerKeys(version: 1, layouts: 5)",
        "(version: 1)",
    ] {
        let (keys, warnings) = PlayerKeys::from_ron(text, &def());
        assert_eq!(keys, PlayerKeys::default(), "{text}");
        assert_eq!(warnings.len(), 1, "{text}: {warnings:?}");
        assert!(
            warnings[0].starts_with("unreadable, using the default keys: "),
            "{warnings:?}"
        );
    }
}

#[test]
fn another_version_gives_the_defaults_and_a_warning() {
    let text = saved("\"Info\": [Some(\"g\")]").replace("version: 1", "version: 2");
    let (keys, warnings) = PlayerKeys::from_ron(&text, &def());
    assert_eq!(keys, PlayerKeys::default());
    assert_eq!(warnings, ["version 2 isn't 1, using the default keys"]);
}

#[test]
fn unknown_layouts_and_actions_are_dropped() {
    let text = "PlayerKeys(version: 1, layouts: {\"Vim\": {}, \"RightHanded\": \
                {\"Attack\": [Some(\"g\")], \"Debug\": [Some(\"h\")], \"Info\": [Some(\"g\")]}})";
    let (b, warnings) = load(text);
    assert_eq!(names(b.slots(Info)), ["g", "-", "-"]);
    assert_eq!(b.keymap(RepeatDef::default()).action(chord("h")), None);
    assert_eq!(
        warnings,
        [
            "RightHanded: unknown action \"Attack\" dropped",
            "RightHanded: unknown action \"Debug\" dropped",
            "unknown layout \"Vim\" dropped",
        ]
    );
}

#[test]
fn a_missing_action_keeps_its_default_slots() {
    let (b, warnings) = load(&saved("\"Info\": [None, Some(\"g\"), None]"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(names(b.slots(Info)), ["-", "g", "-"]);
    assert_eq!(names(b.slots(Confirm)), ["f", "-", "-"]);
}

#[test]
fn bad_and_reserved_chords_and_extra_slots_are_dropped() {
    let (b, warnings) = load(&saved(
        "\"Info\": [Some(\"Nope\"), Some(\"Escape\"), Some(\"g\"), Some(\"h\")]",
    ));
    assert_eq!(names(b.slots(Info)), ["-", "-", "g"]);
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings[0].starts_with("RightHanded: Info: 4 slots, only the first 3 kept"));
    assert!(warnings[1].starts_with("RightHanded: Info: unknown key chord \"Nope\""));
    assert_eq!(
        warnings[2],
        "RightHanded: Info: Escape is a fixed key and can't be bound"
    );
    // Short slot lists are padded with empty slots.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"g\")]"));
    assert!(warnings.is_empty());
    assert_eq!(names(b.slots(Info)), ["g", "-", "-"]);
}

#[test]
fn a_chord_in_two_stored_slots_stays_in_the_first() {
    let (b, warnings) = load(&saved(
        "\"Rewind\": [Some(\"g\")], \"Info\": [Some(\"e\"), Some(\"g\")]",
    ));
    // Action order: Info comes before Rewind, so Info keeps `g`.
    assert_eq!(names(b.slots(Info)), ["e", "g", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
    assert_eq!(
        warnings,
        ["RightHanded: Rewind: g is already on another action"]
    );
}

#[test]
fn a_stored_chord_moves_off_a_default_slot() {
    // Info takes `r`, Rewind's default key; Rewind isn't stored.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"r\")]"));
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(names(b.slots(Info)), ["r", "-", "-"]);
    assert!(b.is_unmapped(Action::Rewind));
}

#[test]
fn a_required_action_left_unmapped_resets_the_layout() {
    // Info takes `f`, Confirm's only default key: Confirm has none left.
    let (b, warnings) = load(&saved("\"Info\": [Some(\"f\")]"));
    assert_eq!(b, right());
    assert_eq!(
        warnings,
        ["RightHanded: Confirm would have no key, using the default keys"]
    );
    // Stored with every slot empty: the same.
    let (b, warnings) = load(&saved("\"Cancel\": [None, None, None], \"EndTurn\": []"));
    assert_eq!(b, right());
    assert_eq!(
        warnings,
        ["RightHanded: Cancel, EndTurn would have no key, using the default keys"]
    );
}

#[test]
fn a_repaired_layout_equal_to_the_defaults_has_no_entry() {
    let (keys, warnings) = PlayerKeys::from_ron(&saved("\"Confirm\": [Some(\"f\")]"), &def());
    assert!(warnings.is_empty());
    assert!(!keys.is_custom(Layout::RightHanded));
}
