//! The player's key and button bindings (tickets 0217 and 0816, ADR-0031,
//! ADR-0050): every rebindable action has [`SLOTS`] key slots and [`SLOTS`]
//! controller-button slots. Each layout keeps its own keys
//! ([`LayoutBindings`]); the buttons are one setup shared by both layouts
//! ([`PadBindings`]); the lot is saved as [`PlayerKeys`]. Rules from
//! `docs/design/controls.md`, *Rebinding keys* and *Rebinding buttons*.
//! Pure: [`Ctx`](crate::screen::Ctx) reads and writes the saved text through
//! `Storage`.
//!
//! Invariants: a chord is in at most one slot of a [`LayoutBindings`], and
//! no [reserved](LayoutBindings::is_reserved) chord is in any; a button is
//! in at most one slot of a [`PadBindings`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{Action, Button, Chord, Keymap, KeymapDef, Layout, RepeatDef, SLOTS};
use crate::screen::DEBUG_TOOLS;

/// One action's key slots; `None` is an empty slot.
pub type Slots = [Option<Chord>; SLOTS];

/// One action's controller-button slots; `None` is an empty slot.
pub type ButtonSlots = [Option<Button>; SLOTS];

/// The saved form of a set of slots: action name → its slots' names.
type Saved = BTreeMap<String, Vec<Option<String>>>;

/// Every rebindable action's slots of keys or of buttons, with the rules
/// both share: a thing is in at most one slot, and binding one that is
/// already in a slot moves it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotTable<T> {
    slots: BTreeMap<Action, [Option<T>; SLOTS]>,
}

impl<T: Copy + Eq> SlotTable<T> {
    /// Every rebindable action, with empty slots.
    fn empty() -> Self {
        Self {
            slots: Action::ALL
                .into_iter()
                .filter(|a| a.is_rebindable())
                .map(|a| (a, [None; SLOTS]))
                .collect(),
        }
    }

    /// `action`'s slots (all empty for an action that has none).
    fn get(&self, action: Action) -> [Option<T>; SLOTS] {
        self.slots.get(&action).copied().unwrap_or([None; SLOTS])
    }

    /// The slot holding `item`, if any.
    fn find(&self, item: T) -> Option<(Action, usize)> {
        self.slots.iter().find_map(|(&action, slots)| {
            slots
                .iter()
                .position(|&c| c == Some(item))
                .map(|i| (action, i))
        })
    }

    /// Whether `action` has a slot `i`.
    fn check(action: Action, i: usize) -> Result<(), BindError> {
        if !action.is_rebindable() {
            return Err(BindError::NotRebindable(action));
        }
        if i >= SLOTS {
            return Err(BindError::NoSuchSlot(i));
        }
        Ok(())
    }

    /// Puts `item` in `action`'s slot `i`, replacing what was there. If it
    /// was in another slot, that slot is emptied and returned.
    fn place(
        &mut self,
        action: Action,
        i: usize,
        item: T,
    ) -> Result<Option<(Action, usize)>, BindError> {
        Self::check(action, i)?;
        let from = self.find(item);
        if from == Some((action, i)) {
            return Ok(None);
        }
        if let Some((other, j)) = from {
            self.clear(other, j);
        }
        self.slots.entry(action).or_insert([None; SLOTS])[i] = Some(item);
        Ok(from)
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    fn clear(&mut self, action: Action, i: usize) {
        if let Some(slot) = self.slots.get_mut(&action).and_then(|s| s.get_mut(i)) {
            *slot = None;
        }
    }

    /// Whether every slot of `action` is empty.
    fn is_unmapped(&self, action: Action) -> bool {
        self.get(action).iter().all(Option::is_none)
    }

    /// The [required](Action::is_required) actions with every slot empty,
    /// in [`Action::ALL`] order.
    fn unmapped_required(&self) -> Vec<Action> {
        Action::ALL
            .into_iter()
            .filter(|&a| a.is_required() && self.is_unmapped(a))
            .collect()
    }

    /// Every filled slot with its action, in action then slot order.
    fn pairs(&self) -> impl Iterator<Item = (T, Action)> + '_ {
        self.slots
            .iter()
            .flat_map(|(&action, slots)| slots.iter().flatten().map(move |&c| (c, action)))
    }
}

impl<T: Copy + fmt::Display> SlotTable<T> {
    /// The saved form: every action with all its slots.
    fn saved(&self) -> Saved {
        self.slots
            .iter()
            .map(|(action, slots)| {
                let names = slots.iter().map(|c| c.map(|c| c.to_string())).collect();
                (action.name().to_owned(), names)
            })
            .collect()
    }
}

/// Why [`LayoutBindings::bind`] or [`PadBindings::bind`] refused.
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
    table: SlotTable<Chord>,
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
            table: SlotTable::empty(),
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
        self.table.get(action)
    }

    /// Whether `chord` can never be put in a slot: a [fixed chord] (plain
    /// `Escape` or `Delete`; their `Shift+` chords are ordinary), or a Debug
    /// chord in a build with debug tools.
    ///
    /// [fixed chord]: Chord::is_reserved
    pub fn is_reserved(&self, chord: Chord) -> bool {
        chord.is_reserved() || (self.debug_reserved && self.debug.contains(&chord))
    }

    /// The slot holding `chord`, if any.
    pub fn find(&self, chord: Chord) -> Option<(Action, usize)> {
        self.table.find(chord)
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
        SlotTable::<Chord>::check(action, i)?;
        self.allowed(chord)?;
        self.table.place(action, i, chord)
    }

    /// [`BindError::Reserved`] if `chord` [is reserved](Self::is_reserved).
    fn allowed(&self, chord: Chord) -> Result<(), BindError> {
        if self.is_reserved(chord) {
            return Err(BindError::Reserved(chord));
        }
        Ok(())
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    pub fn clear(&mut self, action: Action, i: usize) {
        self.table.clear(action, i);
    }

    /// Whether every slot of `action` is empty (shows `! not mapped`).
    pub fn is_unmapped(&self, action: Action) -> bool {
        self.table.is_unmapped(action)
    }

    /// The [required](Action::is_required) actions with no key, in
    /// [`Action::ALL`] order. The Key bindings screen can't be left while
    /// this isn't empty.
    pub fn unmapped_required(&self) -> Vec<Action> {
        self.table.unmapped_required()
    }

    /// The keymap these bindings give: every slot, the Debug chords (unless
    /// a slot took one), and the fixed keys ([`Keymap::new`]). Keys only:
    /// controller buttons don't belong to a layout ([`PadBindings`],
    /// [`Keymap::with_pad`]).
    pub fn keymap(&self, repeat: RepeatDef) -> Keymap {
        let debug = self.debug.iter().map(|&c| (c, Action::Debug));
        // A chord given twice keeps its last action, so a slot beats Debug.
        Keymap::new(debug.chain(self.table.pairs()), repeat)
    }
}

/// The player-edited controller buttons: every rebindable action's button
/// slots. One setup for both keyboard layouts (`docs/design/controls.md`,
/// *Rebinding buttons*); no button is reserved, and both sticks' directions
/// count as buttons.
///
/// The pure editing API for the Key bindings screen (0816), the same shape
/// as [`LayoutBindings`]: [`bind`](Self::bind), [`clear`](Self::clear),
/// [`defaults`](Self::defaults) (restore defaults),
/// [`unmapped_required`](Self::unmapped_required).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadBindings {
    table: SlotTable<Button>,
}

impl PadBindings {
    /// The default buttons from `keymap.ron`'s `pad` table, each action's
    /// buttons in its slots in file order.
    pub fn defaults(def: &KeymapDef) -> Self {
        let mut bindings = Self {
            table: SlotTable::empty(),
        };
        for (&action, buttons) in &def.pad {
            for (i, &button) in buttons.iter().take(SLOTS).enumerate() {
                // As in `LayoutBindings::from_def`.
                bindings.bind(action, i, button).ok();
            }
        }
        bindings
    }

    /// `action`'s button slots (all empty for Debug, which has none).
    pub fn slots(&self, action: Action) -> ButtonSlots {
        self.table.get(action)
    }

    /// The slot holding `button`, if any.
    pub fn find(&self, button: Button) -> Option<(Action, usize)> {
        self.table.find(button)
    }

    /// Puts `button` in `action`'s slot `i`, replacing what was there. If
    /// `button` was in another slot (of any action, including `action`),
    /// that slot is emptied and returned: the button *moves*, which may
    /// leave that action with no button.
    pub fn bind(
        &mut self,
        action: Action,
        i: usize,
        button: Button,
    ) -> Result<Option<(Action, usize)>, BindError> {
        self.table.place(action, i, button)
    }

    /// Empties `action`'s slot `i` (nothing happens for a slot that doesn't
    /// exist).
    pub fn clear(&mut self, action: Action, i: usize) {
        self.table.clear(action, i);
    }

    /// Whether every slot of `action` is empty (shows `! not mapped`).
    pub fn is_unmapped(&self, action: Action) -> bool {
        self.table.is_unmapped(action)
    }

    /// The [required](Action::is_required) actions with no button, in
    /// [`Action::ALL`] order. The Key bindings screen can't be left while
    /// this isn't empty.
    pub fn unmapped_required(&self) -> Vec<Action> {
        self.table.unmapped_required()
    }

    /// Every bound button with its action, for [`Keymap::with_pad`]: in
    /// action then slot order.
    pub fn pairs(&self) -> impl Iterator<Item = (Button, Action)> + '_ {
        self.table.pairs()
    }
}

/// Version written in, and required of, the saved config.
pub const PLAYER_KEYS_VERSION: u32 = 2;

/// The oldest saved version still read: version 1 (ticket 0217) had keys
/// only, and loads with the default buttons.
const OLDEST_READABLE_VERSION: u32 = 1;

/// What repair warnings about the controller buttons start with (a
/// layout's start with its name).
const PAD_WARNING: &str = "controller";

/// Every layout's player bindings and the controller buttons, saved under
/// the `Storage` key [`KEYBINDINGS_KEY`](crate::screen::KEYBINDINGS_KEY). A
/// layout the player hasn't changed has no entry and uses its defaults, and
/// so do the buttons. **Each layout keeps its own keys; the buttons are
/// shared by both.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerKeys {
    layouts: BTreeMap<Layout, LayoutBindings>,
    /// The player's buttons; `None` while they are the defaults.
    pad: Option<PadBindings>,
}

/// The saved form: action and chord names as text, so an unknown name
/// drops only itself on load instead of the whole file.
#[derive(Serialize, Deserialize)]
#[serde(rename = "PlayerKeys")]
struct PlayerKeysFile {
    version: u32,
    layouts: BTreeMap<String, Saved>,
    /// The controller buttons, if the player changed them (since version
    /// 2).
    #[serde(default)]
    pad: Option<Saved>,
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

    /// The controller buttons: the player's, or the defaults if they
    /// haven't changed them. The same whatever the layout.
    pub fn pad_bindings(&self, def: &KeymapDef) -> PadBindings {
        self.pad
            .clone()
            .unwrap_or_else(|| PadBindings::defaults(def))
    }

    /// The keymap for `layout` with the player's keys and buttons.
    pub fn keymap(&self, def: &KeymapDef, layout: Layout) -> Keymap {
        let keys = self.bindings(def, layout).keymap(def.repeat);
        keys.with_pad(self.pad_bindings(def).pairs())
    }

    /// The keymap before any layout is chosen ([`Keymap::layout_picker`]),
    /// with the player's buttons: someone playing only with a controller
    /// never picks a layout.
    pub fn layout_picker_keymap(&self, def: &KeymapDef) -> Keymap {
        Keymap::layout_picker(def).with_pad(self.pad_bindings(def).pairs())
    }

    /// Replaces the controller buttons. Buttons equal to the defaults are
    /// stored as "no entry", as in [`set`](Self::set).
    pub fn set_pad(&mut self, def: &KeymapDef, bindings: PadBindings) {
        self.pad = (bindings != PadBindings::defaults(def)).then_some(bindings);
    }

    /// Whether the player has changed the controller buttons.
    pub fn is_custom_pad(&self) -> bool {
        self.pad.is_some()
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
    /// slots, e.g. `"Confirm": [Some("f"), Some("Enter"), None]`, and the
    /// buttons the same way under `pad` if changed, e.g. `"Confirm":
    /// [Some("South"), None, None]`.
    pub fn to_ron(&self) -> String {
        let layouts = self
            .layouts
            .iter()
            .map(|(layout, bindings)| (layout.name().to_owned(), bindings.table.saved()))
            .collect();
        let file = PlayerKeysFile {
            version: PLAYER_KEYS_VERSION,
            layouts,
            pad: self.pad.as_ref().map(|pad| pad.table.saved()),
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
    /// - unreadable text or an unknown version: defaults for every layout
    ///   and for the buttons;
    /// - version 1 (keys only): the default buttons;
    /// - unknown layouts or actions, unreadable chords or buttons and extra
    ///   slots are dropped;
    /// - reserved chords are dropped;
    /// - a chord in two stored slots stays in the first (in
    ///   [`Action::ALL`] and slot order); a default slot holding a stored
    ///   chord is emptied (the key moves, as with [`LayoutBindings::bind`]);
    /// - an action the layout doesn't list keeps its default slots;
    /// - a layout left with a required action unmapped goes back to its
    ///   defaults;
    /// - the buttons are repaired by the same rules as one layout's keys.
    pub fn from_ron(text: &str, def: &KeymapDef) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let version = match ron::from_str::<VersionOnly>(text) {
            Ok(v) => v.version,
            Err(e) => {
                warnings.push(format!("unreadable, using the default keys: {e}"));
                return (Self::default(), warnings);
            }
        };
        if !(OLDEST_READABLE_VERSION..=PLAYER_KEYS_VERSION).contains(&version) {
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
            let defaults = LayoutBindings::defaults(def, layout);
            let warn = |message: String| warnings.push(format!("{layout}: {message}"));
            let allowed = |chord| defaults.allowed(chord);
            let table = repair(&defaults.table, actions, "key", Chord::parse, allowed, warn);
            keys.set(def, layout, LayoutBindings { table, ..defaults });
        }
        if let Some(actions) = &file.pad {
            let defaults = PadBindings::defaults(def).table;
            let warn = |message: String| warnings.push(format!("{PAD_WARNING}: {message}"));
            let table = repair(
                &defaults,
                actions,
                "button",
                Button::parse,
                |_| Ok(()),
                warn,
            );
            keys.set_pad(def, PadBindings { table });
        }
        (keys, warnings)
    }
}

/// The slots saved as `actions` over `defaults` (see
/// [`PlayerKeys::from_ron`]), calling `warn` for each fix. `what` is what a
/// slot holds ("key" or "button"), `parse` reads one from its saved name
/// and `allowed` refuses the ones that may not be in a slot.
fn repair<T: Copy + Ord + fmt::Display>(
    defaults: &SlotTable<T>,
    actions: &Saved,
    what: &str,
    parse: impl Fn(&str) -> Result<T, String>,
    allowed: impl Fn(T) -> Result<(), BindError>,
    mut warn: impl FnMut(String),
) -> SlotTable<T> {
    let mut stored = BTreeMap::new();
    for (name, slots) in actions {
        match Action::from_name(name).filter(|a| a.is_rebindable()) {
            Some(action) => {
                stored.insert(action, slots);
            }
            None => warn(format!("unknown action \"{name}\" dropped")),
        }
    }
    let mut table = defaults.clone();
    for &action in stored.keys() {
        for i in 0..SLOTS {
            table.clear(action, i);
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
            let item = match parse(name) {
                Ok(item) => item,
                Err(e) => {
                    warn(format!("{action}: {e}"));
                    continue;
                }
            };
            if !claimed.insert(item) {
                warn(format!("{action}: {item} is already on another action"));
                continue;
            }
            if let Err(e) = allowed(item).and_then(|()| table.place(action, i, item)) {
                warn(format!("{action}: {e}"));
            }
        }
    }
    let unmapped = table.unmapped_required();
    if unmapped.is_empty() {
        table
    } else {
        let names: Vec<&str> = unmapped.iter().map(|a| a.name()).collect();
        warn(format!(
            "{} would have no {what}, using the default {what}s",
            names.join(", ")
        ));
        defaults.clone()
    }
}

#[cfg(test)]
mod tests;
