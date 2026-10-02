//! The Preparations screen (ticket 0408, `docs/design/weapons-and-items.md`,
//! *Loadout* and *Battle pack*): before a battle, the player changes the
//! army's loadouts and fills the shared pack from the stock, then starts
//! the battle with `Fight!`.
//!
//! The screen edits a [`Preparations`] (the battle's setup and the units
//! left out of it) through the rules in [`trpg_core::prep`] and hands it
//! back ([`PreparationsScreen::prep`]).
//!
//! Controls, by [`Focus`]:
//!
//! - **Tabs** (`Loadouts`, `Pack`, `Fight!`): Left and Right pick one,
//!   Confirm (or Down) opens it; on `Fight!` Confirm starts the battle.
//!   Cancel does nothing unless the caller allows leaving; then it asks
//!   [`LEAVE_QUESTION`].
//! - **Loadouts**: the units (the ones left out of this battle after the
//!   others, dimmed, so their gear can be traded: Nick, 0408), then the
//!   chosen unit's slots, then the stock for that slot; Confirm goes one list deeper, Cancel one back. Confirm
//!   on a stock item puts it in the slot (what was there goes back to the
//!   stock); items the unit can't use are dimmed with the reason and can't
//!   be taken. The attack speed line shows the change before it is made.
//! - **Pack**: the stock's consumables on the left, the pack on the right;
//!   Left and Right switch sides, Confirm moves one item across. A full
//!   pack takes no more and says so.

use trpg_core::{
    ArmourWeight, BattleSetup, Equipped, GearSlot, ItemDef, ItemId, PrepError, PrepUnit,
    Preparations, StatValue, StockItem, Unit, Unusable, WeaponKind,
};

use super::battle::info::bonus_text;
use super::battle::items::effect_text;
use super::print_centred;
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{SEPARATOR, cursor_keys_name, help_line, key_name};

/// The heading.
pub const HEADING: &str = "Preparations";
/// The question Cancel asks on the tabs, when leaving is allowed.
pub const LEAVE_QUESTION: &str = "Leave preparations?";
/// Shown for an empty slot.
const EMPTY: &str = "-";
/// The stock row that empties the slot.
pub const PUT_BACK: &str = "(put back in stock)";
/// Shown under the slots of a unit left out of the battle.
pub const NOT_IN_BATTLE: &str = "Not in this battle";
/// Said when a slot is chosen and the stock has nothing for it.
pub const NOTHING_IN_STOCK: &str = "Nothing in stock for this slot.";

/// Row of the heading and the tabs.
const TABS_ROW: i32 = 1;
/// The units list.
const UNITS_BOX: Rect = Rect::new(0, 3, 16, 25);
/// The chosen unit's slots.
const SLOTS_BOX: Rect = Rect::new(16, 3, 36, 25);
/// The stock for the chosen slot.
const STOCK_BOX: Rect = Rect::new(52, 3, 48, 25);
/// The stock's consumables (Pack tab).
const SPARE_BOX: Rect = Rect::new(8, 3, 40, 25);
/// The pack (Pack tab).
const PACK_BOX: Rect = Rect::new(52, 3, 40, 25);
/// Row of the message.
const MESSAGE_ROW: i32 = 29;

/// One of the screen's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// The units' weapons, armour and accessories.
    Loadouts,
    /// The shared consumables.
    Pack,
    /// Starts the battle.
    Fight,
}

impl Tab {
    /// Every tab, left to right.
    pub const ALL: [Tab; 3] = [Tab::Loadouts, Tab::Pack, Tab::Fight];

    /// The tab's name.
    pub const fn label(self) -> &'static str {
        match self {
            Tab::Loadouts => "Loadouts",
            Tab::Pack => "Pack",
            Tab::Fight => "Fight!",
        }
    }
}

/// Which list the cursor is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The tabs.
    Tabs,
    /// Loadouts: the units.
    Units,
    /// Loadouts: the chosen unit's slots.
    Slots,
    /// Loadouts: the stock for the chosen slot.
    Stock,
    /// Pack: the stock's consumables.
    Spare,
    /// Pack: the pack.
    Pack,
}

/// How the screen closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepOutcome {
    /// `Fight!`: start the battle with [`PreparationsScreen::prep`].
    Fight,
    /// The player left Preparations.
    Leave,
}

/// One row of the stock list beside the slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockRow {
    /// What Confirm takes; `None` for the row that empties the slot.
    pub item: Option<StockItem>,
    /// The row's text.
    pub text: String,
    /// Why the unit can't use it, if it can't (the row is dimmed).
    pub unusable: Option<Unusable>,
}

/// Why a unit can't use an item, as the player reads it: `needs rank D`.
pub fn reason_text(why: Unusable) -> String {
    match why {
        Unusable::Kind(kind) => format!("can't use {}", kind_plural(kind)),
        Unusable::Rank(rank) => format!("needs rank {rank:?}"),
        Unusable::Armour(weight) => format!("can't wear {} armour", weight_name(weight)),
    }
}

const fn kind_plural(kind: WeaponKind) -> &'static str {
    match kind {
        WeaponKind::Sword => "swords",
        WeaponKind::Spear => "spears",
        WeaponKind::Axe => "axes",
        WeaponKind::Bow => "bows",
        WeaponKind::Gauntlet => "gauntlets",
    }
}

const fn weight_name(weight: ArmourWeight) -> &'static str {
    match weight {
        ArmourWeight::Light => "light",
        ArmourWeight::Medium => "medium",
        ArmourWeight::Heavy => "heavy",
    }
}

/// The index after `i` (`down`) or before it in a list of `len`, wrapping.
fn step(i: usize, len: usize, down: bool) -> usize {
    match (len, down) {
        (0, _) => 0,
        (_, true) => (i + 1) % len,
        (_, false) => (i + len - 1) % len,
    }
}

/// The first row shown of a list of `len` in a window of `rows` rows, so
/// that row `focus` is in it.
fn window_start(focus: usize, len: usize, rows: usize) -> usize {
    (focus + 1)
        .saturating_sub(rows)
        .min(len.saturating_sub(rows))
}

/// Before a battle: loadouts and the pack. See the module docs. Pops with
/// an [`outcome`](Self::outcome).
#[derive(Debug, Clone)]
pub struct PreparationsScreen {
    prep: Preparations,
    /// Whether Cancel on the tabs may leave.
    can_leave: bool,
    tab: Tab,
    focus: Focus,
    /// The chosen unit, among the army's ([`Preparations::units`]).
    unit: usize,
    /// The chosen slot, among the unit's slots.
    slot: usize,
    /// The row of the stock list beside the slots.
    stock: usize,
    /// The row of the stock's consumables.
    spare: usize,
    /// The row of the pack.
    pack: usize,
    /// Why the last Confirm did nothing.
    message: Option<String>,
    /// Whether [`LEAVE_QUESTION`] is open.
    leaving: bool,
    outcome: Option<PrepOutcome>,
}

impl PreparationsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "preparations";

    /// The screen for `prep`, on the `Loadouts` tab. `can_leave`: Cancel
    /// on the tabs offers to leave.
    pub fn new(prep: Preparations, can_leave: bool) -> Self {
        Self {
            prep,
            can_leave,
            tab: Tab::Loadouts,
            focus: Focus::Tabs,
            unit: 0,
            slot: 0,
            stock: 0,
            spare: 0,
            pack: 0,
            message: None,
            leaving: false,
            outcome: None,
        }
    }

    /// The battle as prepared so far.
    pub fn setup(&self) -> &BattleSetup {
        &self.prep.setup
    }

    /// The battle and the units left out of it, as prepared so far.
    pub fn prep(&self) -> &Preparations {
        &self.prep
    }

    /// How the screen closed, once it has popped.
    pub fn outcome(&self) -> Option<PrepOutcome> {
        self.outcome
    }

    /// The tab shown.
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Where the cursor is.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// The message on screen, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether [`LEAVE_QUESTION`] is open.
    pub fn is_asking_leave(&self) -> bool {
        self.leaving
    }

    /// The army's units: the deployed ones, then the ones left out.
    fn units(&self) -> Vec<(PrepUnit, &Unit)> {
        self.prep.units()
    }

    /// The chosen unit.
    fn unit(&self) -> Option<&Unit> {
        self.who().and_then(|who| self.prep.unit(who))
    }

    /// Which unit is chosen.
    fn who(&self) -> Option<PrepUnit> {
        self.units().get(self.unit).map(|(who, _)| *who)
    }

    /// The chosen unit's slots.
    fn slots(&self) -> Vec<GearSlot> {
        self.who()
            .map_or_else(Vec::new, |who| self.prep.gear_slots(who))
    }

    /// The chosen slot.
    fn gear_slot(&self) -> Option<GearSlot> {
        self.slots().get(self.slot).copied()
    }

    /// The name of item `id`.
    fn item_name(&self, id: &ItemId) -> String {
        self.setup()
            .items
            .get(id)
            .map_or_else(|| id.0.clone(), |d| d.name().to_owned())
    }

    /// What `slot` of `unit` holds: its name and, for a weapon, its
    /// durability (`20/25`).
    fn held(&self, unit: &Unit, slot: GearSlot) -> Option<(String, String)> {
        match slot {
            GearSlot::Weapon(s) => {
                let copy = unit.loadout.weapon(s)?;
                let max = self
                    .setup()
                    .items
                    .weapon(&copy.def)
                    .map_or(0, |w| w.durability);
                Some((
                    self.item_name(&copy.def),
                    format!("{:>2}/{max}", copy.durability_left),
                ))
            }
            GearSlot::Armour => {
                let id = unit.loadout.armour.as_ref()?;
                Some((self.item_name(id), String::new()))
            }
            GearSlot::Accessory => {
                let id = unit.loadout.accessory.as_ref()?;
                Some((self.item_name(id), String::new()))
            }
        }
    }

    /// The stock rows for the chosen unit's chosen slot: the row that
    /// empties the slot (if it holds something), then every stock item of
    /// the slot's kind, the ones the unit can't use with the reason.
    pub fn stock_rows(&self) -> Vec<StockRow> {
        let (Some(unit), Some(slot)) = (self.unit(), self.gear_slot()) else {
            return Vec::new();
        };
        let items = &self.prep.setup.items;
        let mut rows = Vec::new();
        if self.held(unit, slot).is_some() {
            rows.push(StockRow {
                item: None,
                text: PUT_BACK.to_owned(),
                unusable: None,
            });
        }
        let row = |item: StockItem, id: &ItemId, name: String, detail: String| {
            let unusable = self.who().and_then(|who| self.prep.unusable(who, id));
            let detail = unusable.map_or(detail, reason_text);
            StockRow {
                item: Some(item),
                text: format!("{name:<15} {detail}"),
                unusable,
            }
        };
        match slot {
            GearSlot::Weapon(_) => {
                for (i, copy) in self.prep.setup.stock.weapons.iter().enumerate() {
                    let Some(def) = items.weapon(&copy.def) else {
                        continue;
                    };
                    let range = if def.min_range == def.max_range {
                        def.min_range.to_string()
                    } else {
                        format!("{}-{}", def.min_range, def.max_range)
                    };
                    let stats = format!(
                        "Mt{:>2} Hit{:>3} Wt{:>2} Rng{range}",
                        def.might, def.hit, def.weight
                    );
                    let detail =
                        format!("{stats:<21} {:>2}/{}", copy.durability_left, def.durability);
                    rows.push(row(
                        StockItem::Weapon(i),
                        &copy.def,
                        def.name.clone(),
                        detail,
                    ));
                }
            }
            GearSlot::Armour | GearSlot::Accessory => {
                for (id, count) in &self.prep.setup.stock.items {
                    let detail = match (slot, items.get(id)) {
                        (GearSlot::Armour, Some(ItemDef::Armour(a))) => {
                            format!("{}  Wt {}", bonus_text(&a.bonus), a.weight)
                        }
                        (GearSlot::Accessory, Some(ItemDef::Accessory(a))) => bonus_text(&a.bonus),
                        _ => continue,
                    };
                    let name = self.item_name(id);
                    rows.push(row(
                        StockItem::Item(id.clone()),
                        id,
                        name,
                        format!("×{count}  {detail}"),
                    ));
                }
            }
        }
        rows
    }

    /// The stock's consumables with how many there are, in id order.
    pub fn spare(&self) -> Vec<(ItemId, u32)> {
        let items = &self.prep.setup.items;
        let stock = self.prep.setup.stock.items.iter();
        stock
            .filter(|(id, _)| items.consumable(id).is_some())
            .map(|(id, n)| (id.clone(), *n))
            .collect()
    }

    /// The pack grouped by item with how many of each, in the order each
    /// was first packed.
    pub fn packed(&self) -> Vec<(ItemId, usize)> {
        let mut groups: Vec<(ItemId, usize)> = Vec::new();
        for id in &self.prep.setup.pack.items {
            match groups.iter_mut().find(|(g, _)| g == id) {
                Some((_, n)) => *n += 1,
                None => groups.push((id.clone(), 1)),
            }
        }
        groups
    }

    /// `Pack 3/6`.
    pub fn pack_header(&self) -> String {
        let pack = &self.prep.setup.pack;
        format!("Pack {}/{}", pack.items.len(), pack.cap)
    }

    /// The army after Confirm on stock row `row`, if that changes it.
    fn after(&self, row: &StockRow) -> Option<Preparations> {
        let (unit, slot) = (self.who()?, self.gear_slot()?);
        let mut after = self.prep.clone();
        let done = match &row.item {
            Some(item) => after.gear_from_stock(unit, slot, item),
            None => after.gear_to_stock(unit, slot),
        };
        done.ok().map(|()| after)
    }

    /// The weapon slot whose attack speed is shown: the chosen slot if the
    /// cursor is on a weapon slot (or in its stock list), else `None` (the
    /// equipped weapon).
    fn speed_slot(&self) -> Option<usize> {
        match (self.focus, self.gear_slot()) {
            (Focus::Slots | Focus::Stock, Some(GearSlot::Weapon(s))) => Some(s),
            _ => None,
        }
    }

    /// The name of the weapon `unit` attacks with from `slot` (`None`: its
    /// equipped weapon or spell); `no weapon` without one.
    fn in_hand(setup: &BattleSetup, unit: &Unit, slot: Option<usize>) -> String {
        let equipped = slot
            .map(Equipped::Weapon)
            .or_else(|| unit.loadout.equipped.clone());
        let name = match equipped {
            Some(Equipped::Weapon(s)) => unit
                .loadout
                .weapon(s)
                .and_then(|w| setup.items.weapon(&w.def))
                .map(|w| w.name.clone()),
            Some(Equipped::Spell(spell)) => setup.spells.get(&spell).map(|s| s.name.clone()),
            None => None,
        };
        name.unwrap_or_else(|| "no weapon".to_owned())
    }

    /// The chosen unit's attack speed line: `AS 6 with Iron Sword`, and
    /// while a stock row is highlighted the change taking it would make,
    /// `AS 6 → 4 with Steel Sword`. The speed is with the weapon of the
    /// highlighted weapon slot in hand, else with the equipped weapon.
    pub fn speed_line(&self) -> Option<String> {
        let (who, unit) = (self.who()?, self.unit()?);
        let slot = self.speed_slot();
        let now: StatValue = self.prep.attack_speed(who, slot)?;
        let highlighted = self.stock_rows().into_iter().nth(self.stock);
        let after = (self.focus == Focus::Stock)
            .then_some(highlighted)
            .flatten()
            .and_then(|row| self.after(&row));
        Some(match after {
            Some(after) => {
                let then = after.attack_speed(who, slot).unwrap_or(now);
                let with = after
                    .unit(who)
                    .map(|u| Self::in_hand(&after.setup, u, slot))
                    .unwrap_or_default();
                format!("AS {now} → {then} with {with}")
            }
            None => format!(
                "AS {now} with {}",
                Self::in_hand(&self.prep.setup, unit, slot)
            ),
        })
    }

    /// The bottom help line.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        let keys = Some(cursor_keys_name(km));
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
        if self.leaving {
            return help_line(&[confirm("yes"), cancel("no")]).replace(SEPARATOR, " / ");
        }
        match self.focus {
            Focus::Tabs => {
                let open = match self.tab {
                    Tab::Fight => "start the battle",
                    _ => "open",
                };
                let leave = (
                    self.can_leave.then(|| key_name(km, Action::Cancel)),
                    "leave",
                );
                help_line(&[(keys, "choose"), confirm(open), leave])
            }
            Focus::Units => help_line(&[(keys, "choose"), confirm("change gear"), cancel("back")]),
            Focus::Slots => help_line(&[(keys, "choose"), confirm("stock"), cancel("back")]),
            Focus::Stock => help_line(&[(keys, "choose"), confirm("take"), cancel("back")]),
            Focus::Spare => help_line(&[(keys, "choose"), confirm("pack it"), cancel("back")]),
            Focus::Pack => help_line(&[(keys, "choose"), confirm("put back"), cancel("back")]),
        }
    }

    /// Opens the shown tab (Confirm or Down on the tabs). Returns whether
    /// the screen is done (`Fight!`).
    fn open_tab(&mut self, ctx: &mut Ctx) -> bool {
        let focus = match self.tab {
            Tab::Loadouts => (!self.units().is_empty()).then_some(Focus::Units),
            Tab::Pack if !self.spare().is_empty() => Some(Focus::Spare),
            Tab::Pack => (!self.packed().is_empty()).then_some(Focus::Pack),
            Tab::Fight => {
                ctx.audio.menu(MenuSound::Select);
                self.outcome = Some(PrepOutcome::Fight);
                return true;
            }
        };
        match focus {
            Some(focus) => {
                self.focus = focus;
                ctx.audio.menu(MenuSound::Select);
            }
            None => ctx.audio.menu(MenuSound::Denied),
        }
        false
    }

    /// Moves a list cursor from `i` in a list of `len`, with the move sound
    /// if it moved.
    fn moved(i: usize, len: usize, down: bool, ctx: &mut Ctx) -> usize {
        let next = step(i, len, down);
        if next != i {
            ctx.audio.menu(MenuSound::Move);
        }
        next
    }

    /// Goes back to `focus` with the cancel sound.
    fn back(&mut self, focus: Focus, ctx: &mut Ctx) {
        self.focus = focus;
        ctx.audio.menu(MenuSound::Cancel);
    }

    /// Confirm on the highlighted stock row: takes it (or empties the slot)
    /// and goes back to the slots; an item the unit can't use is refused.
    fn take(&mut self, ctx: &mut Ctx) {
        let Some(row) = self.stock_rows().into_iter().nth(self.stock) else {
            return;
        };
        if let Some(after) = self.after(&row) {
            self.prep = after;
            self.focus = Focus::Slots;
            self.stock = 0;
            ctx.audio.menu(MenuSound::Select);
            return;
        }
        ctx.audio.menu(MenuSound::Denied);
        if let (Some(why), Some(unit)) = (row.unusable, self.unit()) {
            self.message = Some(format!("{} {}.", unit.name, reason_text(why)));
        }
    }

    /// Confirm on the highlighted consumable of the stock: packs one.
    fn pack_one(&mut self, ctx: &mut Ctx) {
        let Some((item, _)) = self.spare().into_iter().nth(self.spare) else {
            return;
        };
        match self.prep.setup.pack_from_stock(&item) {
            Ok(()) => ctx.audio.menu(MenuSound::Select),
            Err(e) => {
                ctx.audio.menu(MenuSound::Denied);
                if e == PrepError::PackFull {
                    let pack = &self.prep.setup.pack;
                    self.message = Some(format!(
                        "The pack is full ({}/{}).",
                        pack.items.len(),
                        pack.cap
                    ));
                }
            }
        }
        let left = self.spare().len();
        self.spare = self.spare.min(left.saturating_sub(1));
        if left == 0 {
            self.focus = Focus::Pack;
        }
    }

    /// Confirm on the highlighted pack item: puts one back in the stock.
    fn unpack_one(&mut self, ctx: &mut Ctx) {
        let Some((item, _)) = self.packed().into_iter().nth(self.pack) else {
            return;
        };
        if self.prep.setup.pack_to_stock(&item).is_ok() {
            ctx.audio.menu(MenuSound::Select);
        }
        let left = self.packed().len();
        self.pack = self.pack.min(left.saturating_sub(1));
        if left == 0 {
            self.focus = Focus::Spare;
        }
    }

    /// Left or Right on the Pack tab: to the other list, if it has rows.
    fn switch_side(&mut self, ctx: &mut Ctx) {
        let (other, rows) = match self.focus {
            Focus::Spare => (Focus::Pack, self.packed().len()),
            _ => (Focus::Spare, self.spare().len()),
        };
        if rows > 0 {
            self.focus = other;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Handles one action. Returns whether the screen is done.
    fn handle(&mut self, action: Action, ctx: &mut Ctx) -> bool {
        use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};
        if self.leaving {
            match action {
                Confirm => {
                    ctx.audio.menu(MenuSound::Select);
                    self.outcome = Some(PrepOutcome::Leave);
                    return true;
                }
                Cancel => {
                    self.leaving = false;
                    ctx.audio.menu(MenuSound::Cancel);
                }
                _ => {}
            }
            return false;
        }
        if matches!(
            action,
            Cancel | Confirm | CursorDown | CursorLeft | CursorRight | CursorUp
        ) {
            self.message = None;
        }
        let down = action == CursorDown;
        match (self.focus, action) {
            (Focus::Tabs, CursorLeft | CursorRight) => {
                let n = Tab::ALL.len();
                let i = Tab::ALL.iter().position(|&t| t == self.tab).unwrap_or(0);
                self.tab = Tab::ALL[step(i, n, action == CursorRight)];
                ctx.audio.menu(MenuSound::Move);
            }
            (Focus::Tabs, Confirm) => return self.open_tab(ctx),
            (Focus::Tabs, CursorDown) if self.tab != Tab::Fight => return self.open_tab(ctx),
            (Focus::Tabs, Cancel) if self.can_leave => {
                self.leaving = true;
                ctx.audio.menu(MenuSound::Select);
            }

            (Focus::Units, CursorUp | CursorDown) => {
                self.unit = Self::moved(self.unit, self.units().len(), down, ctx);
                self.slot = 0;
            }
            (Focus::Units, Confirm) => {
                self.focus = Focus::Slots;
                self.slot = 0;
                ctx.audio.menu(MenuSound::Select);
            }
            (Focus::Units | Focus::Spare | Focus::Pack, Cancel) => self.back(Focus::Tabs, ctx),

            (Focus::Slots, CursorUp | CursorDown) => {
                self.slot = Self::moved(self.slot, self.slots().len(), down, ctx);
            }
            (Focus::Slots, Confirm) => {
                if self.stock_rows().is_empty() {
                    self.message = Some(NOTHING_IN_STOCK.to_owned());
                    ctx.audio.menu(MenuSound::Denied);
                } else {
                    self.focus = Focus::Stock;
                    self.stock = 0;
                    ctx.audio.menu(MenuSound::Select);
                }
            }
            (Focus::Slots, Cancel) => self.back(Focus::Units, ctx),

            (Focus::Stock, CursorUp | CursorDown) => {
                self.stock = Self::moved(self.stock, self.stock_rows().len(), down, ctx);
            }
            (Focus::Stock, Confirm) => self.take(ctx),
            (Focus::Stock, Cancel) => self.back(Focus::Slots, ctx),

            (Focus::Spare, CursorUp | CursorDown) => {
                self.spare = Self::moved(self.spare, self.spare().len(), down, ctx);
            }
            (Focus::Spare, Confirm) => self.pack_one(ctx),
            (Focus::Pack, CursorUp | CursorDown) => {
                self.pack = Self::moved(self.pack, self.packed().len(), down, ctx);
            }
            (Focus::Pack, Confirm) => self.unpack_one(ctx),
            (Focus::Spare | Focus::Pack, CursorLeft | CursorRight) => self.switch_side(ctx),
            _ => {}
        }
        false
    }

    /// The heading and the tabs: the shown one lit, as a bar while the
    /// cursor is on the tabs.
    fn draw_tabs(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.print(2, TABS_ROW, HEADING, c(UiColor::TextHighlight), black);
        let mut x = 30;
        for tab in Tab::ALL {
            let label = format!(" {} ", tab.label());
            let (fg, bg) = match (tab == self.tab, self.focus == Focus::Tabs) {
                (true, true) => (c(UiColor::PanelBg), c(UiColor::PanelBorderFocus)),
                (true, false) => (c(UiColor::TextHighlight), black),
                (false, _) => (c(UiColor::Text), black),
            };
            buf.print(x, TABS_ROW, &label, fg, bg);
            x += i32::try_from(label.chars().count()).unwrap_or(0) + 4;
        }
    }

    /// A titled box; its border is lit while it has the cursor.
    fn draw_box(ctx: &Ctx, buf: &mut GlyphBuffer, rect: Rect, title: &str, focused: bool) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let border = if focused {
            UiColor::PanelBorderFocus
        } else {
            UiColor::PanelBorder
        };
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Single, c(border), bg);
        let title = format!(" {title} ");
        buf.print(rect.x + 2, rect.y, &title, c(UiColor::TextHighlight), bg);
    }

    /// The rows of a list inside `rect`, scrolled so row `at` shows: each
    /// `(text, enabled)`. Row `at` is a bar while `focused`, lit otherwise
    /// if `marked`; disabled rows are dim.
    fn draw_list(
        ctx: &Ctx,
        buf: &mut GlyphBuffer,
        rect: Rect,
        rows: &[(String, bool)],
        at: usize,
        (focused, marked): (bool, bool),
    ) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let height = usize::try_from(rect.h - 2).unwrap_or(0);
        let start = window_start(at, rows.len(), height);
        let shown = rows.iter().enumerate().skip(start).take(height);
        for (y, (i, (text, enabled))) in (rect.y + 1..).zip(shown) {
            let (fg, row_bg) = match (i == at, enabled) {
                (true, _) if focused => (bg, c(UiColor::PanelBorderFocus)),
                (_, false) => (c(UiColor::TextDim), bg),
                (true, true) if marked => (c(UiColor::TextHighlight), bg),
                (_, true) => (c(UiColor::Text), bg),
            };
            let line = Rect::new(rect.x + 1, y, rect.w - 2, 1);
            buf.fill_rect(line, Cell::new(' ', fg, row_bg));
            buf.print(rect.x + 2, y, text, fg, row_bg);
        }
    }

    /// The Loadouts tab: units, the chosen unit's slots and attack speed,
    /// and the stock for the chosen slot.
    fn draw_loadouts(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let in_tab = self.focus != Focus::Tabs;
        // Units left out of the battle are dimmed.
        let units: Vec<(String, bool)> = self
            .units()
            .iter()
            .map(|(who, u)| (u.name.clone(), matches!(who, PrepUnit::Deployed(_))))
            .collect();
        let on_units = self.focus == Focus::Units;
        Self::draw_box(ctx, buf, UNITS_BOX, "Units", on_units);
        Self::draw_list(ctx, buf, UNITS_BOX, &units, self.unit, (on_units, in_tab));

        let Some(unit) = self.unit() else {
            return;
        };
        let class = self.prep.setup.classes.get(&unit.class);
        let class = class.map_or(unit.class.0.as_str(), |c| c.name.as_str());
        let title = format!("{} · {class}", unit.name);
        let on_slots = self.focus == Focus::Slots;
        Self::draw_box(ctx, buf, SLOTS_BOX, &title, on_slots);
        let slots = self.slots();
        let rows: Vec<(String, bool)> = slots
            .iter()
            .map(|&slot| {
                let label = match slot {
                    GearSlot::Weapon(_) => "Weapon",
                    GearSlot::Armour => "Armour",
                    GearSlot::Accessory => "Accessory",
                };
                let (name, wear) = self
                    .held(unit, slot)
                    .unwrap_or_else(|| (EMPTY.to_owned(), String::new()));
                (format!("{label:<9}  {name:<15} {wear}"), true)
            })
            .collect();
        let marked = self.focus == Focus::Stock;
        Self::draw_list(ctx, buf, SLOTS_BOX, &rows, self.slot, (on_slots, marked));
        if let Some(line) = self.speed_line() {
            let y = SLOTS_BOX.y + 2 + i32::try_from(rows.len()).unwrap_or(0);
            buf.print(SLOTS_BOX.x + 2, y, &line, c(UiColor::Text), bg);
            if matches!(self.who(), Some(PrepUnit::Benched(_))) {
                let dim = c(UiColor::TextDim);
                buf.print(SLOTS_BOX.x + 2, y + 2, NOT_IN_BATTLE, dim, bg);
            }
        }

        let kind = match self.gear_slot() {
            Some(GearSlot::Weapon(_)) => "weapons",
            Some(GearSlot::Armour) => "armour",
            Some(GearSlot::Accessory) | None => "accessories",
        };
        let on_stock = self.focus == Focus::Stock;
        Self::draw_box(ctx, buf, STOCK_BOX, &format!("Stock: {kind}"), on_stock);
        let rows: Vec<(String, bool)> = self
            .stock_rows()
            .into_iter()
            .map(|r| (r.text, r.unusable.is_none()))
            .collect();
        Self::draw_list(ctx, buf, STOCK_BOX, &rows, self.stock, (on_stock, false));
    }

    /// One consumable row: `Potion ×4  Restore 10 HP`.
    fn consumable_row(&self, id: &ItemId, count: usize) -> (String, bool) {
        let effect = self.prep.setup.items.consumable(id).map(|c| c.effect);
        let effect = effect.map(effect_text).unwrap_or_default();
        let name = self.item_name(id);
        (format!("{name:<15} ×{count:<2}  {effect}"), true)
    }

    /// The Pack tab: the stock's consumables and the pack.
    fn draw_pack(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let on_spare = self.focus == Focus::Spare;
        Self::draw_box(ctx, buf, SPARE_BOX, "Stock", on_spare);
        let rows: Vec<(String, bool)> = self
            .spare()
            .iter()
            .map(|(id, n)| self.consumable_row(id, usize::try_from(*n).unwrap_or(usize::MAX)))
            .collect();
        Self::draw_list(ctx, buf, SPARE_BOX, &rows, self.spare, (on_spare, false));
        let on_pack = self.focus == Focus::Pack;
        Self::draw_box(ctx, buf, PACK_BOX, &self.pack_header(), on_pack);
        let rows: Vec<(String, bool)> = self
            .packed()
            .iter()
            .map(|(id, n)| self.consumable_row(id, *n))
            .collect();
        Self::draw_list(ctx, buf, PACK_BOX, &rows, self.pack, (on_pack, false));
    }
}

impl Screen for PreparationsScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            if self.handle(action, ctx) {
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        self.draw_tabs(ctx, buf);
        match self.tab {
            Tab::Loadouts => self.draw_loadouts(ctx, buf),
            Tab::Pack => self.draw_pack(ctx, buf),
            Tab::Fight => {
                let units = self.prep.setup.player_units().count();
                let ready = format!("Units {units}{SEPARATOR}{}", self.pack_header());
                print_centred(buf, 14, &ready, c(UiColor::Text), black);
            }
        }
        let message = if self.leaving {
            Some(LEAVE_QUESTION)
        } else {
            self.message()
        };
        if let Some(message) = message {
            print_centred(buf, MESSAGE_ROW, message, c(UiColor::TextHighlight), black);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests;
