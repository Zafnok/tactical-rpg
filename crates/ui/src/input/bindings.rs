//! The player's key bindings (ticket 0217, ADR-0031): every rebindable
//! action has [`SLOTS`] key slots, each layout keeps its own, and the lot is
//! saved as [`PlayerKeys`]. Rules from `docs/design/controls.md`,
//! *Rebinding keys*. Pure: [`Ctx`](crate::screen::Ctx) reads and writes the
//! saved text through `Storage`.
//!
//! Invariant of a [`LayoutBindings`]: a chord is in at most one slot, and
//! no [reserved](LayoutBindings::is_reserved) chord is in any.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Action, Chord, Keymap, KeymapDef, Layout, RepeatDef, SLOTS};
use crate::screen::DEBUG_TOOLS;

/// One action's key slots; `None` is an empty slot.
pub type Slots = [Option<Chord>; SLOTS];

/// Why [`LayoutBindings::bind`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BindError {
    /// The chord is fixed by the game (`Escape`, `Delete`) or is the Debug
    /// key in a build with debug tools.
    #[error("{0} is a fixed key and can't be bound")]
    Reserved(Chord),
    /// The action can't be rebound (Debug).
    #[error("{0} can't be rebound")]
    NotRebindable(Action),
    /// Slot index past the last slot.
    #[error("there is no key slot {0}")]
    NoSuchSlot(usize),
}

/// One layout's player-edited bindings: every rebindable action's slots,
/// plus the Debug key from `keymap.ron` (not rebindable).
///
/// The pure editing API for the Key bindings screen (0815): [`bind`],
/// [`clear`], [`defaults`] (restore defaults), [`unmapped_required`].
///
/// [`bind`]: Self::bind
/// [`clear`]: Self::clear
/// [`defaults`]: Self::defaults
/// [`unmapped_required`]: Self::unmapped_required
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutBindings {
    /// Every rebindable action → its slots.
    slots: BTreeMap<Action, Slots>,
    /// Debug's chords, as in `keymap.ron`.
    debug: Vec<Chord>,
    /// Whether the Debug chords are reserved (builds with debug tools,
    /// ADR-0023). Without debug tools a player may bind them, and their
    /// slot wins over Debug.
    debug_reserved: bool,
}

impl LayoutBindings {
    /// `layout`'s default keys from `keymap.ron`, each action's chords in
    /// its slots in file order.
    pub fn defaults(def: &KeymapDef, layout: Layout) -> Self {
        Self::from_def(def, layout, DEBUG_TOOLS)
    }

    /// [`defaults`](Self::defaults), with the Debug chords reserved or not.
    pub(crate) fn from_def(def: &KeymapDef, layout: Layout, debug_reserved: bool) -> Self {
        let mut bindings = Self {
            slots: Action::ALL
                .into_iter()
                .filter(|a| a.is_rebindable())
                .map(|a| (a, Slots::default()))
                .collect(),
            debug: def.chords(layout, Action::Debug).to_vec(),
            debug_reserved,
        };
        for (&action, chords) in def.layouts.get(&layout).into_iter().flatten() {
            for (i, &chord) in chords.iter().take(SLOTS).enumerate() {
                // A loaded keymap can't break the invariant; one built by
                // hand is repaired the same way a player's bind would be.
                bindings.bind(action, i, chord).ok();
            }
        }
        bindings
    }

    /// `action`'s slots (all empty for Debug, which has none).
    pub fn slots(&self, action: Action) -> Slots {
        self.slots.get(&action).copied().unwrap_or_default()
    }

    /// Whether `chord` can never be put in a slot: a [reserved key]
    /// (`Escape`, `Delete`, with or without `Shift`), or a Debug chord in a
    /// build with debug tools.
    ///
    /// [reserved key]: super::Key::is_reserved
    pub fn is_reserved(&self, chord: Chord) -> bool {
        chord.key.is_reserved() || (self.debug_reserved && self.debug.contains(&chord))
    }

    /// The slot holding `chord`, if any.
    pub fn find(&self, chord: Chord) -> Option<(Action, usize)> {
        self.slots.iter().find_map(|(&action, slots)| {
            slots
                .iter()
                .position(|&c| c == Some(chord))
                .map(|i| (action, i))
        })
    }

    /// Puts `chord` in `action`'s slot `i`, replacing what was there. If
    /// `chord` was in another slot (of any action, including `action`),
    /// that slot is emptied and returned: the key *moves*
    /// (`docs/design/controls.md`), which may leave that action with no key.
    pub fn bind(
        &mut self,
        action: Action,
        i: usize,
        chord: Chord,
    ) -> Result<Option<(Action, usize)>, BindError> {
        if !action.is_rebindable() {
            return Err(BindError::NotRebindable(action));
        }
        if i >= SLOTS {
            return Err(BindError::NoSuchSlot(i));
        }
        if self.is_reserved(chord) {
            return Err(BindError::Reserved(chord));
        }
        let from = self.find(chord);
        if from == Some((action, i)) {
            return Ok(None);
        }
        if let Some((other, j)) = from {
            self.clear(other, j);
        }
        self.slots.entry(action).or_default()[i] = Some(chord);
        Ok(from)
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    pub fn clear(&mut self, action: Action, i: usize) {
        if let Some(slot) = self.slots.get_mut(&action).and_then(|s| s.get_mut(i)) {
            *slot = None;
        }
    }

    /// Whether every slot of `action` is empty (shows `! not mapped`).
    pub fn is_unmapped(&self, action: Action) -> bool {
        self.slots(action).iter().all(Option::is_none)
    }

    /// The [required](Action::is_required) actions with no key, in
    /// [`Action::ALL`] order. The Key bindings screen can't be left while
    /// this isn't empty.
    pub fn unmapped_required(&self) -> Vec<Action> {
        Action::ALL
            .into_iter()
            .filter(|&a| a.is_required() && self.is_unmapped(a))
            .collect()
    }

    /// The keymap these bindings give: every slot, the Debug chords (unless
    /// a slot took one), and the fixed keys ([`Keymap::new`]).
    pub fn keymap(&self, repeat: RepeatDef) -> Keymap {
        let debug = self.debug.iter().map(|&c| (c, Action::Debug));
        let slotted = self
            .slots
            .iter()
            .flat_map(|(&action, slots)| slots.iter().flatten().map(move |&c| (c, action)));
        // A chord given twice keeps its last action, so a slot beats Debug.
        Keymap::new(debug.chain(slotted), repeat)
    }
}

/// Version written in, and required of, the saved config.
pub const PLAYER_KEYS_VERSION: u32 = 1;

/// Every layout's player bindings, saved under the `Storage` key
/// [`KEYBINDINGS_KEY`](crate::screen::KEYBINDINGS_KEY). A layout the player
/// hasn't changed has no entry and uses its defaults. **Each layout keeps
/// its own keys.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerKeys {
    layouts: BTreeMap<Layout, LayoutBindings>,
}

/// The saved form: action and chord names as text, so an unknown name
/// drops only itself on load instead of the whole file.
#[derive(Serialize, Deserialize)]
#[serde(rename = "PlayerKeys")]
struct PlayerKeysFile {
    version: u32,
    layouts: BTreeMap<String, BTreeMap<String, Vec<Option<String>>>>,
}

/// Just the version, read first so a future format still gives a clear
/// warning.
#[derive(Deserialize)]
#[serde(rename = "PlayerKeys")]
struct VersionOnly {
    version: u32,
}

impl PlayerKeys {
    /// `layout`'s bindings: the player's, or the defaults if they haven't
    /// changed that layout.
    pub fn bindings(&self, def: &KeymapDef, layout: Layout) -> LayoutBindings {
        self.layouts
            .get(&layout)
            .cloned()
            .unwrap_or_else(|| LayoutBindings::defaults(def, layout))
    }

    /// The keymap for `layout` with the player's bindings.
    pub fn keymap(&self, def: &KeymapDef, layout: Layout) -> Keymap {
        self.bindings(def, layout).keymap(def.repeat)
    }

    /// Replaces `layout`'s bindings; the other layout is untouched.
    /// Bindings equal to the defaults are stored as "no entry", so a later
    /// change to the default keys reaches a player who restored them.
    pub fn set(&mut self, def: &KeymapDef, layout: Layout, bindings: LayoutBindings) {
        if bindings == LayoutBindings::defaults(def, layout) {
            self.layouts.remove(&layout);
        } else {
            self.layouts.insert(layout, bindings);
        }
    }

    /// Whether the player has changed `layout`'s keys.
    pub fn is_custom(&self, layout: Layout) -> bool {
        self.layouts.contains_key(&layout)
    }

    /// The saved form (RON): every changed layout with all its actions'
    /// slots, e.g. `"Confirm": [Some("f"), Some("Enter"), None]`.
    pub fn to_ron(&self) -> String {
        let layouts = self
            .layouts
            .iter()
            .map(|(layout, bindings)| {
                let actions = bindings
                    .slots
                    .iter()
                    .map(|(action, slots)| {
                        let names = slots.iter().map(|c| c.map(|c| c.to_string())).collect();
                        (action.name().to_owned(), names)
                    })
                    .collect();
                (layout.name().to_owned(), actions)
            })
            .collect();
        let file = PlayerKeysFile {
            version: PLAYER_KEYS_VERSION,
            layouts,
        };
        let config = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .compact_arrays(true);
        // Strings, maps and options always serialise.
        ron::ser::to_string_pretty(&file, config).unwrap_or_default()
    }

    /// Reads the saved form, repairing rather than failing, and returns
    /// the result with a warning for everything it had to fix:
    ///
    /// - unreadable text or another version: defaults for every layout;
    /// - unknown layouts or actions, unreadable chords and extra slots are
    ///   dropped;
    /// - reserved chords are dropped;
    /// - a chord in two stored slots stays in the first (in
    ///   [`Action::ALL`] and slot order); a default slot holding a stored
    ///   chord is emptied (the key moves, as with [`LayoutBindings::bind`]);
    /// - an action the layout doesn't list keeps its default slots;
    /// - a layout left with a required action unmapped goes back to its
    ///   defaults.
    pub fn from_ron(text: &str, def: &KeymapDef) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let version = match ron::from_str::<VersionOnly>(text) {
            Ok(v) => v.version,
            Err(e) => {
                warnings.push(format!("unreadable, using the default keys: {e}"));
                return (Self::default(), warnings);
            }
        };
        if version != PLAYER_KEYS_VERSION {
            warnings.push(format!(
                "version {version} isn't {PLAYER_KEYS_VERSION}, using the default keys"
            ));
            return (Self::default(), warnings);
        }
        let file = match ron::from_str::<PlayerKeysFile>(text) {
            Ok(file) => file,
            Err(e) => {
                warnings.push(format!("unreadable, using the default keys: {e}"));
                return (Self::default(), warnings);
            }
        };
        let mut keys = Self::default();
        for (name, actions) in &file.layouts {
            let Some(layout) = Layout::from_name(name) else {
                warnings.push(format!("unknown layout \"{name}\" dropped"));
                continue;
            };
            let bindings = repair(def, layout, actions, &mut warnings);
            keys.set(def, layout, bindings);
        }
        (keys, warnings)
    }
}

/// `layout`'s bindings from its saved `actions` (see
/// [`PlayerKeys::from_ron`]), adding a warning for each fix.
fn repair(
    def: &KeymapDef,
    layout: Layout,
    actions: &BTreeMap<String, Vec<Option<String>>>,
    warnings: &mut Vec<String>,
) -> LayoutBindings {
    let defaults = LayoutBindings::defaults(def, layout);
    let mut warn = |message: String| warnings.push(format!("{layout}: {message}"));
    let mut stored = BTreeMap::new();
    for (name, slots) in actions {
        match Action::from_name(name).filter(|a| a.is_rebindable()) {
            Some(action) => {
                stored.insert(action, slots);
            }
            None => warn(format!("unknown action \"{name}\" dropped")),
        }
    }
    let mut bindings = defaults.clone();
    for &action in stored.keys() {
        for i in 0..SLOTS {
            bindings.clear(action, i);
        }
    }
    let mut claimed = BTreeSet::new();
    for (&action, slots) in &stored {
        if slots.len() > SLOTS {
            warn(format!(
                "{action}: {} slots, only the first {SLOTS} kept",
                slots.len()
            ));
        }
        for (i, name) in slots.iter().enumerate().take(SLOTS) {
            let Some(name) = name else { continue };
            let chord = match Chord::parse(name) {
                Ok(chord) => chord,
                Err(e) => {
                    warn(format!("{action}: {e}"));
                    continue;
                }
            };
            if !claimed.insert(chord) {
                warn(format!("{action}: {chord} is already on another action"));
                continue;
            }
            if let Err(e) = bindings.bind(action, i, chord) {
                warn(format!("{action}: {e}"));
            }
        }
    }
    let unmapped = bindings.unmapped_required();
    if unmapped.is_empty() {
        bindings
    } else {
        let names: Vec<&str> = unmapped.iter().map(|a| a.name()).collect();
        warn(format!(
            "{} would have no key, using the default keys",
            names.join(", ")
        ));
        defaults
    }
}

#[cfg(test)]
mod tests;
