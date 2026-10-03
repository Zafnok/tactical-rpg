//! The battle screen (ADR-0018, `docs/design/look-and-feel.md`): the map
//! viewport on the left, the side panel on the right and the help bar at the
//! bottom. The player browses with the cursor (ticket 0402), selects a unit,
//! steers its path, moves it and picks an action (0403, [`mode`]), and
//! attacks: picks a weapon and a target, reads the [`forecast`] and watches
//! the combat's [`playback`] (0404), or casts a spell on an enemy, an ally
//! or a tile ([`magic`], 0410); a tile whose terrain changes flashes.
//! Around that (0405): the unit
//! [`info`] screen, the danger zone, the [`map_menu`], ending the turn (and
//! auto-end), and the phase and outcome [`banner`]s. Every command sent is
//! kept in a [`BattleHistory`], and Rewind opens the [`rewind`] screen
//! (0307). One-time [`tips`] pop up over it the first time something new
//! happens (0406). The battle's [`notes`] (0411) show in a box at its start
//! and on the map menu's `Objective` page. Scenes the battle's triggers fire (0705) play as a
//! [`DialogueScreen`] overlay over the map, in order with the banners, or
//! inside a combat's playback (a boss's line before the combat, a death
//! quote before the fall). In the Enemy and Other phases the AI acts
//! ([`ai_phase`], 0502): each action is shown one at a time, and the player
//! can only speed it up or skip fights.
//!
//! The map itself isn't drawn here (ADR-0038): the screen says what is on it
//! ([`BattleScreen::scene`]) and the map skin paints that. Panels, menus,
//! boxes and the help bar are this screen's own cells.

pub mod ai_phase;
pub mod art_list;
pub mod attack;
pub mod banner;
pub mod camera;
pub mod cursor;
pub mod event_sounds;
pub mod forecast;
pub mod info;
pub mod items;
pub mod layout;
pub mod magic;
pub mod map_menu;
pub mod mode;
pub mod notes;
pub mod panel;
pub mod path;
pub mod playback;
pub mod progress;
pub mod rewind;
pub mod skills;
mod sounds;
pub mod tips;
pub mod units;

use std::collections::VecDeque;
use trpg_content::{Content, TipTrigger, battle_campaign};

use trpg_core::lead::DEFAULT_NAME;
use trpg_core::{
    BattleHistory, BattleState, CastTarget, Command, Equipped, Event, Faction, GameMode,
    LeadGender, LeadProfile, Phase, Pos, StatValue, TileSet, Unit, UnitId, danger_zone,
    next_command,
};

use self::ai_phase::{AiAction, PACING};
use self::banner::{Banner, BannerKind};

use self::attack::{Targeting, aimed_first, aimed_options};
use self::camera::Camera;
use self::cursor::Cursor;
use self::event_sounds::CueQueue;
use self::layout::{HELP_BAR, HELP_ROW, MAP_VIEW, SIDE_PANEL};
use self::mode::{Effect, MenuEntry, Mode, Selection};
use self::playback::{Playback, TIMINGS};
use self::progress::{PROGRESS_TIMINGS, Progress};
use self::rewind::{RewindEffect, RewindScreen};
use self::tips::TipState;
use super::OptionsScreen;
use super::dialogue::DialogueScreen;
use super::draw_debug_hint;
use crate::audio::{CURSOR_MOVE, MenuSound};
#[cfg(test)]
use crate::color::Rgb;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::map_view::{CursorView, MapScene, RangeKind, UnitView, default_skin};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::tips::{draw_tip, fill_placeholders};
use crate::widgets::help::{HelpKeys, SEPARATOR, cursor_keys_name, help_line, key_name};

/// Battle id of the debug Quick Battle (`assets/battles/quick.ron`).
pub const QUICK_BATTLE: &str = "quick";

/// The debug Quick Battle (`assets/battles/quick.ron`, which the title
/// screen plays through the game flow) at its start, with its own
/// characters in Classic: `test_small.map` with the placeholder characters
/// (a caster among them, unit 8) against generic enemies (rout), one
/// brigand close enough to fight on turn 1, a Frost Elemental holding its
/// tile, and a trigger of each kind about a rogue who arrives on turn 2.
/// Fails with a message if the content lacks it.
pub fn quick_battle(content: &Content) -> Result<BattleState, String> {
    let def = content
        .battles
        .get(QUICK_BATTLE)
        .ok_or_else(|| format!("no battle \"{QUICK_BATTLE}\""))?;
    let lead = LeadProfile::new(DEFAULT_NAME, LeadGender::Male);
    let campaign = battle_campaign(content, def, GameMode::Classic, lead);
    Ok(BattleState::new(campaign.battle_setup(def, &content.tables())).0)
}

/// How long the `Auto-end: ON/OFF` message stays, in seconds. *Tunable.*
pub const TOAST_S: f32 = 1.5;

/// How long a tile flashes when its terrain changes, in seconds. *Tunable.*
pub const TERRAIN_FLASH_S: f32 = 0.4;

/// A tile whose terrain just changed (a tile cast, a fire burning out): it
/// flashes for [`TERRAIN_FLASH_S`] seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainFlash {
    /// The tile.
    pub pos: Pos,
    /// Seconds it has flashed.
    pub t: f32,
}

impl TerrainFlash {
    /// How strong the flash is now: `1` at the change, fading to none.
    pub fn strength(&self) -> f32 {
        (1.0 - self.t / TERRAIN_FLASH_S).clamp(0.0, 1.0)
    }
}

/// What a [`HealPopup`] tells of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    /// HP restored: `+10`, in `hp_high`.
    Heal,
    /// HP lost to a fire burning out under the unit: `-5`, in `hp_low`.
    Burn,
}

/// A `+10` shown over a unit that was healed, or a `-5` over one a fire
/// burnt, for [`Timings::heal_popup`](playback::Timings::heal_popup)
/// seconds (a burn's start once no banner is on screen).
#[derive(Debug, Clone, PartialEq)]
pub struct HealPopup {
    /// The tile it floats over.
    pub pos: Pos,
    /// HP restored or lost.
    pub amount: StatValue,
    /// Whether the HP was restored or lost.
    pub kind: PopupKind,
    /// Seconds it has been up.
    pub t: f32,
}

impl HealPopup {
    /// What it reads: `+10` for a heal, `-5` for a burn.
    pub fn text(&self) -> String {
        match self.kind {
            PopupKind::Heal => format!("+{}", self.amount),
            PopupKind::Burn => format!("-{}", self.amount),
        }
    }

    /// Its colour: `hp_high` for a heal, `hp_low` for a burn.
    pub fn color(&self) -> UiColor {
        match self.kind {
            PopupKind::Heal => UiColor::HpHigh,
            PopupKind::Burn => UiColor::HpLow,
        }
    }
}

/// The tiles units hostile to the player could attack this turn
/// ([`danger_zone`] with each unit's attack ranges); empty if it can't be
/// worked out.
pub fn danger_tiles(state: &BattleState) -> TileSet {
    let tiles = &state.map().tiles;
    danger_zone(
        state.map(),
        state.terrain(),
        state.classes(),
        state.units(),
        Faction::Player,
        |u| u.attack_ranges(state.classes(), state.items(), state.spells()),
    )
    .unwrap_or_else(|_| TileSet::new(tiles.width(), tiles.height()))
}

/// Something the battle screen shows once no combat is playing, one at a
/// time in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Queued {
    /// The battle's notes (0411), at its start: closed by Confirm.
    Notes,
    /// A phase or outcome banner.
    Banner(Banner),
    /// A scene a trigger fired, by id: played as a [`DialogueScreen`]
    /// overlay.
    Scene(String),
}

/// Why the battle screen closes before the battle is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leaving {
    /// `Restart Battle`: the game flow starts the battle again.
    Restart,
    /// `Suspend`: the game flow saves the battle and goes back to the
    /// title.
    Suspend,
}

/// The battle screen: the player browses the map with the cursor, selects
/// and moves units ([`Mode`]), opens the map menu, ends the turn; phase and
/// outcome banners show as the battle goes on, and the outcome's closes the
/// screen. Rewind while browsing in the player phase opens the
/// [`RewindScreen`]. In the Enemy and Other phases, once their banner has
/// closed, the AI's actions play one by one ([`AiAction`]), then the phase
/// ends.
#[derive(Debug, Clone)]
pub struct BattleScreen {
    state: BattleState,
    history: BattleHistory,
    camera: Camera,
    /// The viewport's size in tiles, across and down, that the camera is
    /// kept for: the map skin's, taken at the start of every frame.
    view: (i32, i32),
    cursor: Cursor,
    mode: Mode,
    /// The danger zone, while shown (recomputed after every command).
    danger: Option<TileSet>,
    /// Auto-end: end the player phase when its last unit has acted. Off by
    /// default (`docs/design/turn-structure.md`, ticket 0420). The
    /// player's setting ([`Settings::auto_end_turn`]), read at the start
    /// of every frame.
    ///
    /// [`Settings::auto_end_turn`]: crate::settings::Settings::auto_end_turn
    auto_end: bool,
    /// Whether a fight's playback is shown, or skipped as the Cancel key
    /// skips one: the player's Combat animations setting, read at the
    /// start of every frame.
    show_fights: bool,
    /// A command was applied: auto-end is checked once the screen is back
    /// to browsing.
    end_armed: bool,
    /// A short message and the seconds it has left.
    toast: Option<(String, f32)>,
    /// Banners and scenes waiting to be shown, in the order of their
    /// events, the first on screen (after a combat's playback).
    queue: VecDeque<Queued>,
    rewind: Option<RewindScreen>,
    /// One-time tips waiting to show (0406).
    tips: TipState,
    /// Heal numbers floating over units.
    popups: Vec<HealPopup>,
    /// Tiles flashing after their terrain changed.
    flashes: Vec<TerrainFlash>,
    /// The EXP bar and level-up pages of the last command (0602), shown
    /// once its combat has played.
    progress: Option<Progress>,
    /// Sounds of events that no playback or walk plays (0424), waiting
    /// for their moment.
    cues: CueQueue,
    /// The unit whose walk was shown since the last command: its move's
    /// steps have been heard.
    walked: Option<UnitId>,
    /// The player chose `Restart Battle` or `Suspend`: the screen closes,
    /// and the game flow does it.
    leaving: Option<Leaving>,
    /// Where the cursor and camera were when the player phase ended: they
    /// go back there when the next one starts.
    player_view: Option<(Pos, Camera)>,
    /// Seconds the battle notes have been up (0411): the units they are
    /// about blink.
    notes_t: f32,
}

impl BattleScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "battle";

    /// A screen showing `state` (a battle just started: its history, and
    /// every rewind charge, start here), with the cursor on the first player lord
    /// (else the first player unit, else the map's centre) and the camera
    /// centred on it, in a view as big as the default map skin shows (the
    /// first frame fits it to the skin in use).
    pub fn new(state: BattleState) -> Self {
        let tiles = &state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        let player = |u: &&Unit| u.faction == Faction::Player;
        let units = state.units();
        let start = units
            .iter()
            .filter(player)
            .find(|u| u.is_lord)
            .or_else(|| units.iter().find(player))
            .map_or_else(|| Pos::new(i32::from(w) / 2, i32::from(h) / 2), |u| u.pos);
        let view = default_skin().view_tiles(MAP_VIEW);
        Self {
            camera: Camera::centred_on(start, w, h, view),
            view,
            cursor: Cursor::new(start),
            mode: Mode::after_command(&state),
            history: BattleHistory::new(state.clone()),
            rewind: None,
            state,
            danger: None,
            auto_end: false,
            show_fights: true,
            end_armed: false,
            toast: None,
            queue: VecDeque::new(),
            tips: TipState::default(),
            popups: vec![],
            flashes: vec![],
            progress: None,
            cues: CueQueue::default(),
            walked: None,
            leaving: None,
            player_view: None,
            notes_t: 0.0,
        }
    }

    /// [`BattleScreen::new`] for a battle just started with `events`: its
    /// notes (0411) show first, if it has any and isn't already decided,
    /// then the banners of the events (turn 1's `PLAYER PHASE`; the outcome
    /// of a battle decided at its start) and the scenes they fire (a
    /// turn-1 trigger), in the order of the events.
    pub fn start(state: BattleState, events: &[Event]) -> Self {
        let mut screen = Self::new(state);
        if !screen.state.battle_notes().is_empty() && screen.state.outcome().is_none() {
            screen.queue.push_back(Queued::Notes);
        }
        screen.queue.extend(events.iter().filter_map(|e| match e {
            Event::SceneTriggered { scene } => Some(Queued::Scene(scene.clone())),
            _ => Banner::for_event(e).map(Queued::Banner),
        }));
        screen
    }

    /// A suspended battle carrying on (0802): the battle as `history`'s
    /// commands left it, with its rewind points and the charges left.
    /// `history` must have its content tables
    /// ([`BattleHistory::restore_tables`]).
    pub fn resume(history: BattleHistory) -> Self {
        let mut screen = Self::new(history.state_at(history.len()));
        screen.history = history;
        screen
    }

    /// The scene waiting to play next, once no combat is playing, no EXP or
    /// level up is shown and no banner is before it.
    pub fn queued_scene(&self) -> Option<&str> {
        if self.playing() || self.progress.is_some() {
            return None;
        }
        match self.queue.front() {
            Some(Queued::Scene(id)) => Some(id),
            _ => None,
        }
    }

    /// The heal numbers on screen.
    pub fn popups(&self) -> &[HealPopup] {
        &self.popups
    }

    /// The tiles flashing after their terrain changed.
    pub fn flashes(&self) -> &[TerrainFlash] {
        &self.flashes
    }

    /// Whether auto-end is on.
    pub fn auto_end(&self) -> bool {
        self.auto_end
    }

    /// The danger zone, while shown.
    pub fn danger(&self) -> Option<&TileSet> {
        self.danger.as_ref()
    }

    /// The EXP bar or level-up page on screen, if any: once the command's
    /// combat has played.
    pub fn progress(&self) -> Option<&Progress> {
        if self.playing() {
            return None;
        }
        self.progress.as_ref()
    }

    /// Gives `action` to the EXP bar or level-up page on screen, if any
    /// (Confirm or Cancel finishes or closes a page; other keys wait). Returns
    /// whether it took it.
    fn progress_key(&mut self, action: Action) -> bool {
        if self.playing() {
            return false;
        }
        let Some(progress) = self.progress.as_mut() else {
            return false;
        };
        if matches!(action, Action::Confirm | Action::Cancel) {
            progress.confirm();
        }
        if progress.done() {
            self.progress = None;
        }
        true
    }

    /// Plays the EXP bar or level-up page on screen for `dt` seconds.
    fn tick_progress(&mut self, dt: f32, confirm_held: bool) {
        if self.playing() {
            return;
        }
        if let Some(progress) = self.progress.as_mut() {
            progress.tick(dt, confirm_held);
            if progress.done() {
                self.progress = None;
            }
        }
    }

    /// Whether a combat's playback or an AI action is playing: banners,
    /// scenes, tips and EXP wait for it.
    fn playing(&self) -> bool {
        matches!(self.mode, Mode::Combat(_) | Mode::AiAction(_))
    }

    /// The banner on screen, if any: the first waiting, once no combat or
    /// AI action is playing and no EXP or level up is shown.
    pub fn banner(&self) -> Option<&Banner> {
        if self.playing() || self.progress.is_some() {
            return None;
        }
        match self.queue.front() {
            Some(Queued::Banner(b)) => Some(b),
            _ => None,
        }
    }

    /// Whether the battle's notes are on screen in their box (at the start
    /// of the battle, until Confirm closes it).
    pub fn notes_open(&self) -> bool {
        self.queue.front() == Some(&Queued::Notes)
    }

    /// Whether the battle's notes are on screen, in their box or on the
    /// `Objective` page: the units they are about blink.
    fn notes_shown(&self) -> bool {
        self.notes_open() || self.mode == Mode::Objective
    }

    /// Gives `action` to the tip or the battle notes box on screen, if any:
    /// each takes the keys until it is closed (a tip by Confirm or Cancel,
    /// the notes by Confirm). Returns whether one took it.
    fn overlay_key(&mut self, action: Action) -> bool {
        if let Some(tip) = self.shown_tip() {
            if matches!(action, Action::Confirm | Action::Cancel) {
                self.tips.dismiss(tip);
            }
            return true;
        }
        if self.notes_open() {
            if action == Action::Confirm {
                self.queue.pop_front();
            }
            return true;
        }
        false
    }

    /// Counts the `dt` seconds the battle notes have been up, from 0 each
    /// time they come up.
    fn tick_notes(&mut self, dt: f32) {
        self.notes_t = if self.notes_shown() {
            self.notes_t + dt
        } else {
            0.0
        };
    }

    /// The top row of a box `h` rows tall listing the battle notes: off
    /// the rows of the units they are about ([`notes::box_top`]), where the
    /// map skin has their tiles of `scene`.
    fn notes_top(&self, ctx: &Ctx, scene: &MapScene, h: i32) -> i32 {
        let noted = self.state.battle_notes();
        let rows: Vec<i32> = self
            .state
            .units()
            .iter()
            .filter(|u| notes::is_noted(noted, u.id))
            .filter_map(|u| ctx.map_skin.tile_cells(scene, MAP_VIEW, self.drawn_pos(u)))
            .map(|cells| cells.y)
            .collect();
        notes::box_top(h, &rows)
    }

    /// The message shown for a moment, if any.
    pub fn toast(&self) -> Option<&str> {
        self.toast.as_ref().map(|(t, _)| t.as_str())
    }

    /// Shows or hides the danger zone.
    fn toggle_danger(&mut self) {
        self.danger = match self.danger {
            Some(_) => None,
            None => Some(danger_tiles(&self.state)),
        };
    }

    /// Flips auto-end, saves it with the other options and says so.
    fn toggle_auto_end(&mut self, ctx: &mut Ctx) {
        self.auto_end = !self.auto_end;
        let on = self.auto_end;
        // If saving fails it still holds for this session.
        let _ = ctx.change_settings(|s| s.auto_end_turn = on);
        let text = format!("Auto-end: {}", on_off(self.auto_end));
        self.toast = Some((text, TOAST_S));
    }

    /// A player unit's level-up in `events` fires [`TipTrigger::FirstLevelUp`].
    fn note_level_ups(&mut self, events: &[Event]) {
        let player_level_up = events.iter().any(|e| {
            matches!(e, Event::LeveledUp { unit, .. }
                if self.state.unit(*unit).is_some_and(|u| u.faction == Faction::Player))
        });
        if player_level_up {
            self.tips.fire(TipTrigger::FirstLevelUp);
        }
    }

    /// Fires the tip triggers that show in the battle's current state (the
    /// ones that come from a command's events are fired as it applies).
    fn detect_tips(&mut self) {
        let state = &self.state;
        if state.turn() == 1 && state.phase() == Phase::Player {
            self.tips.fire(TipTrigger::FirstBattleStart);
        }
        if state.phase() == Phase::Enemy {
            self.tips.fire(TipTrigger::FirstEnemyPhase);
        }
        let low = |u: &Unit| u.faction == Faction::Player && u.hp > 0 && u.hp * 2 <= u.stats.hp;
        if state.units().iter().any(low) {
            self.tips.fire(TipTrigger::FirstLowHp);
        }
        match &self.mode {
            Mode::Selected(sel) => {
                self.tips.fire(TipTrigger::FirstUnitSelected);
                let reachable = |u: &&Unit| {
                    u.faction != Faction::Player
                        && (sel.moves.contains(u.pos) || sel.attack.contains(u.pos))
                };
                if state.units().iter().any(|u| reachable(&u)) {
                    self.tips.fire(TipTrigger::FirstEnemyInRange);
                }
            }
            Mode::Targeting(_) => self.tips.fire(TipTrigger::FirstForecast),
            Mode::Idle { .. } => {
                let on_enemy = self.hovered().is_some_and(|u| u.faction == Faction::Enemy);
                if on_enemy && self.danger.is_none() {
                    self.tips.fire(TipTrigger::DangerZoneAvailable);
                }
            }
            _ => {}
        }
    }

    /// The tip on screen, if any: the first queued whose moment has come.
    /// Tips wait out a combat's playback, a scene, the battle notes and the
    /// rewind screen;
    /// the enemy-phase tip shows as that phase begins (over its banner),
    /// the others once a player phase is browsing without a banner.
    pub fn shown_tip(&self) -> Option<TipTrigger> {
        if self.rewind.is_some()
            || self.playing()
            || self.progress.is_some()
            || self.queued_scene().is_some()
            || self.notes_open()
        {
            return None;
        }
        self.tips.queued().find(|&t| {
            t == TipTrigger::FirstEnemyPhase
                || (self.state.phase() == Phase::Player && self.banner().is_none())
        })
    }

    /// Closes the banner on screen. An outcome's leaves the battle.
    fn close_banner(&mut self) -> Transition {
        let Some(&banner) = self.banner() else {
            return Transition::None;
        };
        self.queue.pop_front();
        match banner.kind {
            BannerKind::Outcome(_) => Transition::Pop,
            BannerKind::Phase { .. } => Transition::None,
        }
    }

    /// Whether the AI is playing its phase (Enemy or Other): the player
    /// can only speed it up or skip its fights.
    pub fn ai_phase(&self) -> bool {
        self.state.phase() != Phase::Player
    }

    /// In an AI phase, once nothing else is on screen (banner, scene, tip,
    /// EXP, a previous action), shows the AI's next action, or ends the
    /// phase when it has none.
    fn drive_ai(&mut self, ctx: &Ctx) {
        if !self.ai_phase() || self.state.outcome().is_some() {
            return;
        }
        // An AI unit that may move after its attack: the AI decides that,
        // not the player.
        if matches!(self.mode, Mode::MoveAfter { .. }) {
            self.mode = Mode::default();
        }
        if !matches!(self.mode, Mode::Idle { .. })
            || !self.queue.is_empty()
            || self.progress.is_some()
            || self.shown_tip().is_some()
        {
            return;
        }
        match next_command(&self.state, &ctx.content.ai) {
            Some(cmd) => self.apply_ai(&cmd),
            None => {
                self.apply(&Command::EndPhase);
            }
        }
    }

    /// During an AI action, the camera pans to its unit, then follows its
    /// walk; the cursor (hidden) goes with it, so the side panel shows it.
    fn follow_ai_action(&mut self) {
        let Mode::AiAction(a) = &self.mode else {
            return;
        };
        if a.walking() {
            let at = a.walker_pos();
            self.follow(at);
            if self.cursor.pos != at {
                self.cursor.jump(at);
            }
        } else {
            self.camera.origin = a.camera();
        }
    }

    /// Applies the AI's `cmd` and shows it ([`AiAction`]): the camera pans
    /// to its unit, the cursor marks it, it walks its move, then the
    /// command's combat plays, if any.
    fn apply_ai(&mut self, cmd: &Command) {
        let before = self.state.units().to_vec();
        let unit = match cmd {
            Command::Act { unit, .. } | Command::MoveAfter { unit, .. } => Some(*unit),
            _ => None,
        };
        // Its steps are heard as the walk is shown.
        self.walked = unit;
        let Some(events) = self.apply(cmd) else {
            return;
        };
        let Some(unit) = unit else {
            return;
        };
        // An AI command moves only its own unit.
        let path = events
            .iter()
            .find_map(|e| match e {
                Event::UnitMoved { path, .. } => Some(path.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let Some(start) = before.iter().find(|u| u.id == unit).map(|u| u.pos) else {
            return;
        };
        let from = self.camera.origin;
        self.follow(start);
        let to = self.camera.origin;
        self.camera.origin = from;
        self.cursor.jump(start);
        let then = std::mem::take(&mut self.mode);
        let action = AiAction::new(unit, before, (from, to), path, then, PACING);
        self.mode = Mode::AiAction(Box::new(action));
    }

    /// The scene to play now, taken from the combat's playback where its
    /// clock has stopped, or from the front of the queue. A scene missing
    /// from the content (validation rules it out) is dropped.
    fn next_scene(&mut self, ctx: &Ctx) -> Option<trpg_content::Scene> {
        let id = if let Mode::Combat(pb) = &mut self.mode {
            pb.take_scene()?
        } else {
            self.queued_scene()?;
            match self.queue.pop_front() {
                Some(Queued::Scene(id)) => id,
                _ => return None,
            }
        };
        ctx.content.dialogue.get(&id).cloned()
    }

    /// With auto-end on, ends the player phase once the screen is back to
    /// browsing after the command that left no player unit ready.
    fn check_auto_end(&mut self) {
        if !self.end_armed
            || !matches!(self.mode, Mode::Idle { .. })
            || !self.queue.is_empty()
            || self.progress.is_some()
        {
            return;
        }
        self.end_armed = false;
        let player = self.state.phase() == Phase::Player;
        if self.auto_end
            && player
            && self.state.outcome().is_none()
            && map_menu::ready_players(&self.state) == 0
        {
            self.apply(&Command::EndPhase);
        }
    }

    /// Whether the player chose `Restart Battle` (the screen then pops,
    /// and the game flow starts the battle again).
    pub fn restart_requested(&self) -> bool {
        self.leaving == Some(Leaving::Restart)
    }

    /// Whether the player chose `Suspend` (the screen then pops, and the
    /// game flow saves the battle and goes back to the title).
    pub fn suspend_requested(&self) -> bool {
        self.leaving == Some(Leaving::Suspend)
    }

    /// The suspend save couldn't be written: the battle goes on, showing
    /// `message`.
    pub fn suspend_failed(&mut self, message: String) {
        self.leaving = None;
        self.toast = Some((message, TOAST_S));
    }

    /// Applies `cmd` as if the player (or the AI) had sent it: scripted
    /// tests play a battle with it.
    #[cfg(any(test, feature = "harness"))]
    pub fn send(&mut self, cmd: &Command) {
        self.apply(cmd);
    }

    /// The commands sent so far and the rewind charges left.
    pub fn history(&self) -> &BattleHistory {
        &self.history
    }

    /// The rewind screen, if open.
    pub fn rewind(&self) -> Option<&RewindScreen> {
        self.rewind.as_ref()
    }

    /// What the player is doing.
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// The battle shown.
    pub fn state(&self) -> &BattleState {
        &self.state
    }

    /// The camera.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// The cursor.
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Scrolls the camera to keep `target` [`Camera::MARGIN`] tiles from the
    /// viewport's edges.
    pub fn follow(&mut self, target: Pos) {
        let tiles = &self.state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        self.camera.follow(target, w, h, self.view, Camera::MARGIN);
    }

    /// The camera for a viewport of `view` tiles: the screen's own, or, if
    /// that was kept for another size (the map skin changed), one centred on
    /// the cursor.
    fn camera_in(&self, view: (i32, i32)) -> Camera {
        if view == self.view {
            return self.camera;
        }
        let tiles = &self.state.map().tiles;
        Camera::centred_on(self.cursor.pos, tiles.width(), tiles.height(), view)
    }

    /// Starts a frame of `dt` seconds: keeps the cameras for as many tiles
    /// as the map skin shows ([`refit`](Self::refit)), takes the player's
    /// settings as they are now (the Options screen may have changed
    /// them), and advances the cursor's pulse.
    fn begin_frame(&mut self, ctx: &Ctx, dt: f32) {
        self.auto_end = ctx.settings().auto_end_turn;
        self.show_fights = ctx.settings().combat_animations;
        let view = ctx.map_skin.view_tiles(MAP_VIEW);
        if view != self.view {
            self.refit(view);
        }
        self.cursor.tick(dt);
    }

    /// The map skin changed (a debug tool) and the viewport now shows
    /// `view` tiles: the camera centres on the cursor, the camera kept for
    /// the player's next phase centres on the cursor kept with it, and an
    /// AI action's pan is worked out again, taking as long as before.
    fn refit(&mut self, view: (i32, i32)) {
        let tiles = &self.state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        self.camera = self.camera_in(view);
        self.view = view;
        if let Some((pos, camera)) = &mut self.player_view {
            *camera = Camera::centred_on(*pos, w, h, view);
        }
        if let Mode::AiAction(a) = &mut self.mode {
            let mut to = self.camera;
            to.follow(a.start(), w, h, view, Camera::MARGIN);
            a.repan((self.camera.origin, to.origin));
        }
    }

    /// The unit drawn under the cursor, if any.
    pub fn hovered(&self) -> Option<&Unit> {
        let pos = self.cursor.pos;
        self.state.units().iter().find(|u| self.drawn_pos(u) == pos)
    }

    /// Where `unit` is drawn: its tile, or where it walks or stands before
    /// its move is sent ([`Mode::drawn_pos`]).
    fn drawn_pos(&self, unit: &Unit) -> Pos {
        self.mode.drawn_pos(unit.id).unwrap_or(unit.pos)
    }

    /// Applies `cmd` (built by [`mode::step`] from legal choices) and
    /// records it in the history, then plays its combat if it had one, and
    /// continues browsing (or with the unit's move after its attack); queues
    /// the banners of its events, and its scenes unless its combat plays
    /// them, and updates the danger zone. Leaving the player phase keeps
    /// where the cursor and camera were; they go back there when it comes
    /// round again. Returns the events; a refused command changes nothing
    /// and returns `None`.
    fn apply(&mut self, cmd: &Command) -> Option<Vec<Event>> {
        let before = self.state.units().to_vec();
        let was_player = !self.ai_phase();
        let walked = self.walked.take();
        // Refused: the battle is unchanged and the player browses again.
        let events = self.state.apply(cmd).ok();
        let playback = events
            .as_ref()
            .and_then(|events| Playback::new(events, &before, self.state.fallen(), TIMINGS));
        if let Some(events) = &events {
            self.history.push(cmd.clone());
            self.note_level_ups(events);
            let in_playback = playback.is_some();
            self.queue.extend(events.iter().filter_map(|e| match e {
                Event::SceneTriggered { scene } if !in_playback => {
                    Some(Queued::Scene(scene.clone()))
                }
                _ => Banner::for_event(e).map(Queued::Banner),
            }));
            self.popups.extend(events.iter().filter_map(|e| match *e {
                Event::Healed { target, amount } if amount > 0 => {
                    let pos = self.state.unit(target)?.pos;
                    Some(HealPopup {
                        pos,
                        amount,
                        kind: PopupKind::Heal,
                        t: 0.0,
                    })
                }
                Event::BurnDamage { pos, amount, .. } if amount > 0 => Some(HealPopup {
                    pos,
                    amount,
                    kind: PopupKind::Burn,
                    t: 0.0,
                }),
                _ => None,
            }));
            self.flashes.extend(events.iter().filter_map(|e| match *e {
                Event::TerrainChanged { pos, .. } => Some(TerrainFlash { pos, t: 0.0 }),
                _ => None,
            }));
            if self.danger.is_some() {
                self.danger = Some(danger_tiles(&self.state));
            }
            // Checked (phase, units ready) once back to browsing.
            self.end_armed = true;
            self.progress = Progress::new(events, &before, &self.state, PROGRESS_TIMINGS);
            if was_player && self.ai_phase() {
                self.player_view = Some((self.cursor.pos, self.camera));
            } else if !was_player
                && !self.ai_phase()
                && let Some((pos, camera)) = self.player_view.take()
            {
                self.cursor.jump(pos);
                self.camera = camera;
            }
        }
        let played = events.clone();
        let playback = events.and_then(|events| {
            let banner = art_list::playback_banner(&self.state, &events, &before);
            let playback = playback.map(|p| p.with_banner(banner));
            let cues = event_sounds::event_cues(&events, walked, playback.is_some(), &self.state);
            self.cues.extend(cues);
            let attacks = event_sounds::combat_attacks(&events, &before, &self.state);
            playback.map(|p| p.with_sounds(&attacks, event_sounds::heals(&events)))
        });
        self.mode = match playback {
            Some(mut p) => {
                if !self.show_fights {
                    // Combat animations are off.
                    p.skip();
                }
                Mode::Combat(Box::new(p))
            }
            None => Mode::after_command(&self.state),
        };
        played
    }

    /// Plays this frame's sounds (0424): the combat playback's and the
    /// walk's that the coming tick of `dt` seconds reaches (`held`:
    /// Confirm is down), and the queued events' that are due.
    fn play_sounds(&mut self, ctx: &mut Ctx, dt: f32, held: bool) {
        let mut due = match &self.mode {
            Mode::Combat(playback) => playback.sounds(dt, held),
            _ => Vec::new(),
        };
        if let Some((id, tiles)) = self.mode.tiles_entered(dt, held) {
            self.walked = Some(id);
            let step = self
                .state
                .unit(id)
                .and_then(|u| event_sounds::unit_step_sound(&self.state, u));
            due.extend(step.into_iter().cycle().take(tiles));
        }
        due.extend(self.cues.tick(dt));
        for cue in due {
            ctx.audio.play_sound(cue);
        }
    }

    /// Applies `cmd`, which leaves the unit's action open (`Equip`), and
    /// reopens the action menu for the unit at the end of its path (its
    /// ranges recomputed: the weapon changed), on `Equip`.
    fn apply_stay(&mut self, cmd: &Command, sel: Selection) {
        // Back on what was chosen (`Talk` is gone once its talk played).
        let entry = match cmd {
            Command::Talk { .. } => MenuEntry::Talk,
            _ => MenuEntry::Equip,
        };
        self.apply(cmd);
        let Some(mut fresh) = Selection::new(&self.state, sel.unit) else {
            return;
        };
        if self.state.unit(sel.unit).is_some_and(|u| u.acted) {
            return;
        }
        fresh.path = sel.path;
        self.mode = mode::back_to_entry(fresh, &self.state, entry);
    }

    /// The units as drawn, each with how far it has faded out: the
    /// battle's units, except during a combat's playback, where its
    /// fighters show the HP it has reached (the attacker not dimmed yet)
    /// and the units that fell stay until they have faded; and during an
    /// AI action, before its combat, the units as they were before it.
    pub fn shown_units(&self) -> Vec<(Unit, f32)> {
        if let Mode::AiAction(a) = &self.mode {
            return a.units().into_iter().map(|u| (u, 0.0)).collect();
        }
        let Mode::Combat(pb) = &self.mode else {
            return self
                .state
                .units()
                .iter()
                .map(|u| (u.clone(), 0.0))
                .collect();
        };
        let attacker = pb.bouts().first().map(|b| b.attacker.unit);
        self.state
            .units()
            .iter()
            .chain(pb.falls())
            .map(|u| {
                let mut u = u.clone();
                if let Some(hp) = pb.hp(u.id) {
                    u.hp = hp;
                }
                if Some(u.id) == attacker {
                    u.acted = false;
                }
                let fade = pb.fade(u.id).unwrap_or(0.0);
                (u, fade)
            })
            .collect()
    }

    /// Whether Rewind opens the rewind screen: browsing in the player phase
    /// of a battle still running.
    fn can_open_rewind(&self) -> bool {
        matches!(self.mode, Mode::Idle { .. })
            && self.state.phase() == Phase::Player
            && self.state.outcome().is_none()
    }

    /// Handles one action on the open rewind screen.
    fn rewind_step(&mut self, ctx: &mut Ctx, action: Action) {
        let Some(screen) = self.rewind.as_mut() else {
            return;
        };
        let before = (screen.focused().map(|e| e.point), screen.is_confirming());
        let effect = screen.step(action);
        let after = (screen.focused().map(|e| e.point), screen.is_confirming());
        let sound = match effect {
            RewindEffect::None if before == after => None,
            // Up and down move the highlight; Confirm and Cancel open and
            // close the prompt.
            RewindEffect::None if before.1 == after.1 => Some(MenuSound::Move),
            RewindEffect::None if after.1 => Some(MenuSound::Select),
            RewindEffect::None | RewindEffect::Close => Some(MenuSound::Cancel),
            RewindEffect::Rewind(_) => Some(MenuSound::Select),
        };
        if let Some(sound) = sound {
            ctx.audio.menu(sound);
        }
        match effect {
            RewindEffect::None => {}
            RewindEffect::Close => self.rewind = None,
            RewindEffect::Rewind(point) => {
                // The screen only confirms a listed point with a charge left.
                if let Ok(state) = self.history.rewind_to(point) {
                    self.state = state;
                    self.mode = Mode::after_command(&self.state);
                    self.queue.clear();
                    self.progress = None;
                    self.end_armed = false;
                    if self.danger.is_some() {
                        self.danger = Some(danger_tiles(&self.state));
                    }
                }
                self.rewind = None;
            }
        }
    }

    /// Gives `action` to the mode ([`mode::step`], after [`Mode::route`]
    /// applies the optional split keys), plays its menu sound and carries
    /// out its effect. Returns whether the map menu's `Options` was chosen:
    /// the frame then opens the Options screen.
    fn step_mode(&mut self, ctx: &mut Ctx, action: Action) -> bool {
        let Some(action) = self.mode.route(action, &ctx.keymap) else {
            return false;
        };
        let before = self.mode.clone();
        let mode = std::mem::take(&mut self.mode);
        let (mode, effect) = mode::step(mode, action, self.cursor.pos, &self.state);
        if let Some(sound) = sounds::step_sound(action, &before, &mode, &effect) {
            ctx.audio.menu(sound);
        }
        self.mode = mode;
        match effect {
            Effect::None => {}
            Effect::Apply(cmd) => {
                self.apply(&cmd);
            }
            Effect::ApplyStay(cmd, sel) => self.apply_stay(&cmd, *sel),
            Effect::Cursor(to) => {
                self.cursor.jump(to);
                self.follow(to);
            }
            Effect::Restart => self.leaving = Some(Leaving::Restart),
            Effect::Suspend => self.leaving = Some(Leaving::Suspend),
            Effect::Options => return true,
        }
        false
    }

    /// Plays the frame's sounds and advances what is animated by `dt`
    /// seconds at the player's speeds (0805; `held`: Confirm is down): a
    /// walk, a fight's playback, an AI action, the EXP bar.
    fn animate(&mut self, ctx: &mut Ctx, dt: f32, held: bool) {
        let dt = dt * ctx.settings().battle_speed(self.ai_phase());
        self.play_sounds(ctx, dt, held);
        let mode = std::mem::take(&mut self.mode);
        self.mode = mode.tick(dt, held, &self.state);
        self.follow_ai_action();
        // A walk aimed at an enemy (0428) ends in its forecast: the cursor
        // goes onto the target.
        if let Mode::Targeting(t) = &self.mode
            && let Some(at) = self.state.unit(t.target()).map(|u| u.pos)
            && self.cursor.pos != at
        {
            self.cursor.jump(at);
            self.follow(at);
        }
        self.tick_progress(dt, held);
    }

    /// Moves the cursor one tile for a cursor key; the camera and a
    /// selected unit's path follow. Returns whether it moved (not at the
    /// map's edge).
    fn move_cursor(&mut self, action: Action) -> bool {
        let tiles = &self.state.map().tiles;
        let (w, h) = (tiles.width(), tiles.height());
        let moved = self.cursor.step(action, w, h);
        if moved {
            let to = self.cursor.pos;
            self.follow(to);
            self.mode.cursor_moved(to, &self.state);
        }
        moved
    }

    /// Whether `unit` can still act this phase: its faction's phase and it
    /// hasn't acted.
    fn is_ready(&self, unit: &Unit) -> bool {
        Phase::of(unit.faction) == self.state.phase() && !unit.acted
    }

    /// The acting faction's ready units, ordered by `(y, x)`.
    pub fn ready_units(&self) -> Vec<Pos> {
        let mut ready: Vec<Pos> = self
            .state
            .units()
            .iter()
            .filter(|u| self.is_ready(u))
            .map(|u| u.pos)
            .collect();
        ready.sort_by_key(|p| (p.y, p.x));
        ready
    }

    /// Moves the cursor (and camera) to the next ready unit after the
    /// cursor in `(y, x)` order, or the previous one before it; wraps
    /// around. Nothing happens with no ready units.
    fn cycle(&mut self, forward: bool) {
        let ready = self.ready_units();
        let here = (self.cursor.pos.y, self.cursor.pos.x);
        let key = |p: &&Pos| (p.y, p.x);
        let to = if forward {
            ready.iter().find(|p| key(p) > here).or(ready.first())
        } else {
            ready.iter().rev().find(|p| key(p) < here).or(ready.last())
        };
        if let Some(&to) = to {
            self.cursor.jump(to);
            self.follow(to);
        }
    }

    /// The help line of a tip, the battle notes, EXP or level-up page, or
    /// banner on screen.
    fn overlay_help(&self, km: HelpKeys<'_>) -> Option<String> {
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        if self.shown_tip().is_some() || self.notes_open() {
            return Some(help_line(&[confirm("close")]));
        }
        if let Some(p) = self.progress() {
            return Some(progress::help(p, km));
        }
        let label = match self.banner()?.kind {
            BannerKind::Outcome(_) => "continue",
            BannerKind::Phase { .. } => "skip",
        };
        Some(help_line(&[confirm(label)]))
    }

    /// The help line for the mode and what is under the cursor, e.g. `f
    /// select · e info · s next unit · r rewind · d back` over a ready unit while
    /// browsing. Key names come from the keymap.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        let keys = Some(cursor_keys_name(km));
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        let select = |label| (Some(key_name(km, km.select_action())), label);
        let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
        if let Some(r) = &self.rewind {
            return rewind_help(r, ctx);
        }
        let info = (Some(key_name(km, Action::Info)), "info");
        let next = (Some(key_name(km, Action::NextUnit)), "next unit");
        let moves = (keys.clone(), "move");
        if let Some(line) = self.overlay_help(km) {
            return line;
        }
        match &self.mode {
            Mode::Idle { threat } => {
                let back = cancel(if threat.is_some() {
                    "hide range"
                } else {
                    "menu"
                });
                self.help_idle(ctx, info, back)
            }
            Mode::Objective => help_line(&[cancel("back")]),
            Mode::RestartPrompt => help_line(&[confirm("restart"), cancel("back")]),
            Mode::SuspendPrompt => help_line(&[confirm("suspend"), cancel("back")]),

            Mode::EndTurnPrompt { .. } => {
                let [accept, also] = km.end_turn_accept_actions();
                let yes = |a| (Some(key_name(km, a)), "yes");
                help_line(&[yes(accept), yes(also), cancel("no")])
            }
            Mode::Info { .. } => {
                let prev = (Some(key_name(km, Action::PrevUnit)), "previous");
                help_line(&[next, prev, cancel("close")])
            }
            Mode::Selected(sel) => {
                let aimed = sel
                    .target
                    .and_then(|t| self.state.unit(t))
                    .is_some_and(|u| u.pos == self.cursor.pos);
                if aimed {
                    // The forecast opens with a weapon or a spell (0430).
                    let options = sel.target.map_or_else(Vec::new, |t| {
                        aimed_options(&self.state, sel.unit, sel.dest(), t)
                    });
                    let verb = match aimed_first(&self.state, sel.unit, &options) {
                        Some(Equipped::Spell(_)) => "cast",
                        _ => "attack",
                    };
                    help_line(&[moves, select(verb), cancel("cancel")])
                } else if self.cursor.pos == sel.dest() && sel.reach.is_stoppable(sel.dest()) {
                    help_line(&[moves, select("move here"), cancel("cancel")])
                } else {
                    help_line(&[moves, cancel("cancel")])
                }
            }
            Mode::Moving { .. } => help_line(&[confirm("skip")]),
            Mode::ActionMenu { .. }
            | Mode::WeaponMenu { .. }
            | Mode::SkillMenu { .. }
            | Mode::ItemMenu { .. }
            | Mode::MapMenu { .. }
            | Mode::UnitList { .. } => {
                help_line(&[(keys, "choose"), confirm("confirm"), cancel("back")])
            }
            Mode::SpellMenu { .. } => {
                help_line(&[(keys, "choose"), confirm("cast"), cancel("back")])
            }
            Mode::CastTarget(t) => match t.forecast() {
                Some(forecast) => Self::help_targeting(ctx, forecast),
                None => help_line(&[(keys, "next target"), select("cast"), cancel("back")]),
            },
            Mode::EquipMenu { .. } => {
                help_line(&[(keys, "choose"), confirm("equip"), cancel("back")])
            }
            Mode::ItemTarget(_) | Mode::SkillTarget(_) => {
                help_line(&[(keys, "next target"), select("use"), cancel("back")])
            }
            Mode::TalkTarget { .. } => {
                help_line(&[(keys, "next target"), select("talk"), cancel("back")])
            }
            Mode::Targeting(t) => Self::help_targeting(ctx, t),
            Mode::Combat(_) => {
                let hold = Some(format!("hold {}", key_name(km, Action::Confirm)));
                help_line(&[cancel("skip"), (hold, "fast")])
            }
            Mode::AiAction(_) => {
                let hold = Some(format!("hold {}", key_name(km, Action::Confirm)));
                help_line(&[(hold, "fast")])
            }
            Mode::MoveAfter { unit, tiles } => {
                let here = self.state.unit(*unit).map(|u| u.pos);
                if here == Some(self.cursor.pos) {
                    help_line(&[moves, select("stay")])
                } else if tiles.contains(&self.cursor.pos) {
                    help_line(&[moves, select("move here")])
                } else {
                    help_line(&[moves])
                }
            }
        }
    }

    /// The help line while picking an attack's target: left/right pick the
    /// target and up/down the line of the arts list, if it is shown (0414).
    /// Confirm reads `cast` for a spell.
    fn help_targeting(ctx: &Ctx, t: &Targeting) -> String {
        let km = ctx.help_keys();
        let verb = match t.with {
            Equipped::Weapon(_) => "attack",
            Equipped::Spell(_) => "cast",
        };
        let confirm = (Some(key_name(km, Action::Confirm)), verb);
        let cancel = (Some(key_name(km, Action::Cancel)), "back");
        let pair = |a, b| Some(format!("{}/{}", key_name(km, a), key_name(km, b)));
        if t.can_swap() {
            // Left and right swap the weapon or spell of a forecast opened
            // by pointing (0430); the unit keys then change the target.
            let mut entries = vec![
                (pair(Action::CursorLeft, Action::CursorRight), "swap"),
                (pair(Action::PrevUnit, Action::NextUnit), "target"),
            ];
            if t.has_list() {
                entries.push((pair(Action::CursorUp, Action::CursorDown), "art"));
            }
            return help_line(&[entries, vec![confirm, cancel]].concat());
        }
        if !t.has_list() {
            return help_line(&[(Some(cursor_keys_name(km)), "next target"), confirm, cancel]);
        }
        help_line(&[
            (pair(Action::CursorLeft, Action::CursorRight), "target"),
            (pair(Action::CursorUp, Action::CursorDown), "art"),
            confirm,
            cancel,
        ])
    }

    /// The help line while browsing, over what is under the cursor.
    fn help_idle(
        &self,
        ctx: &Ctx,
        info: (Option<String>, &str),
        back: (Option<String>, &str),
    ) -> String {
        let km = ctx.help_keys();
        let select = |label| (Some(key_name(km, km.select_action())), label);
        let next = (Some(key_name(km, Action::NextUnit)), "next unit");
        let moves = (Some(cursor_keys_name(km)), "move");
        let end = (Some(key_name(km, Action::EndTurn)), "end turn");
        let rewind = (
            self.can_open_rewind().then(|| key_name(km, Action::Rewind)),
            "rewind",
        );
        match self.hovered() {
            Some(u) if self.is_ready(u) && u.faction == Faction::Player => {
                help_line(&[select("select"), info, next, rewind, back, end])
            }
            Some(u) if u.faction != Faction::Player => {
                help_line(&[moves, select("range"), info, next, rewind, back, end])
            }
            Some(_) => help_line(&[moves, info, next, rewind, back, end]),
            None => help_line(&[moves, select("menu"), next, rewind, back, end]),
        }
    }

    /// The toggles' state for the message row: the Danger zone key and
    /// `danger zone: OFF`, then the Auto-end key and `auto-end: ON`, each
    /// key named from the active keymap.
    pub fn status(&self, ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        let danger = format!("danger zone: {}", on_off(self.danger.is_some()));
        let auto = format!("auto-end: {}", on_off(self.auto_end));
        help_line(&[
            (Some(key_name(km, Action::DangerZone)), &danger),
            (Some(key_name(km, Action::ToggleAutoEnd)), &auto),
        ])
    }

    /// What is on the visible map (ADR-0038), for the map skin to paint and
    /// for tests to read: the terrain, the tiles flashing after their
    /// terrain changed, the ranges the mode shows, the units as drawn, the
    /// cursor and a selected unit's path. On the rewind screen: the map as
    /// it was just before the highlighted action (as it is now with nothing
    /// listed), with no flashes, ranges, cursor or path.
    pub fn scene(&self, ctx: &Ctx) -> MapScene {
        let size = ctx.map_skin.view_tiles(MAP_VIEW);
        let origin = self.camera_in(size).origin;
        if let Some(r) = &self.rewind {
            let shown = r.focused().map_or(&self.state, |e| &e.before);
            let mut scene = terrain_scene(shown, origin, size);
            for unit in shown.units() {
                scene.push_unit(UnitView::of(unit));
            }
            return scene;
        }
        let mut scene = terrain_scene(&self.state, origin, size);
        for flash in &self.flashes {
            if let Some(tile) = scene.tile_mut(flash.pos) {
                tile.flashes.push(flash.strength());
            }
        }
        self.add_ranges(&mut scene);
        self.add_units(&mut scene);
        scene.cursor = self.cursor_view(ctx).filter(|c| scene.contains(c.pos));
        if let Mode::Selected(sel) = &self.mode {
            scene.path.clone_from(&sel.path);
        }
        scene
    }

    /// Lays the danger zone on `scene`, if shown, then the mode's ranges
    /// over it: a selected unit's move and attack ranges, a shown threat
    /// area, where a unit may move after its attack, or the targets being
    /// chosen from. While a spell's target is picked (0410): the enemies it
    /// can hit as an attack range or the allies it can heal as a heal
    /// range, and each tile it can change marked with the terrain it would
    /// become (Nick: units and terrain must look different).
    fn add_ranges(&self, scene: &mut MapScene) {
        let all = |set: &TileSet| -> Vec<Pos> { set.iter().collect() };
        if let Some(zone) = &self.danger {
            scene.tint(all(zone), RangeKind::Danger);
        }
        match &self.mode {
            Mode::Selected(sel) => {
                scene.tint(all(&sel.moves), RangeKind::Move);
                scene.tint(all(&sel.attack), RangeKind::Attack);
            }
            Mode::Idle {
                threat: Some(threat),
            } => scene.tint(all(&threat.area), RangeKind::Attack),
            Mode::MoveAfter { tiles, .. } => scene.tint(tiles.clone(), RangeKind::Move),
            Mode::Targeting(t) => {
                let at = t.targets.iter().filter_map(|&id| self.state.unit(id));
                scene.tint(at.map(|u| u.pos), RangeKind::Attack);
            }
            Mode::CastTarget(t) => {
                let units = t.targets().iter().filter_map(|target| match target {
                    CastTarget::Unit(id) => self.state.unit(*id).map(|u| u.pos),
                    CastTarget::Tile(_) => None,
                });
                let attack = self
                    .state
                    .spells()
                    .get(t.spell())
                    .is_some_and(trpg_core::SpellDef::is_attack);
                let kind = if attack {
                    RangeKind::Attack
                } else {
                    RangeKind::Heal
                };
                scene.tint(units, kind);
                for (pos, becomes) in t.tile_changes(&self.state) {
                    if let Some(tile) = scene.tile_mut(pos) {
                        tile.becomes = Some(becomes);
                    }
                }
            }
            Mode::SkillTarget(t) => {
                let at = t.targets().iter().filter_map(|&id| self.state.unit(id));
                scene.tint(at.map(|u| u.pos), RangeKind::Attack);
            }
            Mode::ItemTarget(t) => {
                let at = t.targets().iter().filter_map(|&id| self.state.unit(id));
                let dest = t.sel.dest();
                let tiles = at.map(|u| if u.id == t.sel.unit { dest } else { u.pos });
                scene.tint(tiles, RangeKind::Heal);
            }
            _ => {}
        }
    }

    /// Adds the units to `scene`, each where it is drawn; during a combat's
    /// playback or an AI action, as [`shown_units`](Self::shown_units) has
    /// them. While the battle notes are up, the units they are about blink:
    /// they are highlighted in the on half of the blink.
    fn add_units(&self, scene: &mut MapScene) {
        if !self.playing() {
            let blink = self.notes_shown() && notes::blink_on(self.notes_t);
            for unit in self.state.units() {
                let noted = blink && notes::is_noted(self.state.battle_notes(), unit.id);
                let view = UnitView::of(unit).at(self.drawn_pos(unit));
                scene.push_unit(view.highlighted(noted));
            }
            return;
        }
        for (unit, fade) in self.shown_units() {
            scene.push_unit(UnitView::of(&unit).fading(fade));
        }
    }

    /// The cursor as the mode shows it (in the player's cursor style): on
    /// its tile while browsing or on a selected unit; none on the tile the
    /// path's arrowhead points at (a unit selected and the cursor away from
    /// it), during a walk or in a menu. During an AI action, on its unit
    /// until it walks.
    fn cursor_view(&self, ctx: &Ctx) -> Option<CursorView> {
        let pos = self.cursor.pos;
        match &self.mode {
            Mode::Moving { .. }
            | Mode::ActionMenu { .. }
            | Mode::WeaponMenu { .. }
            | Mode::SpellMenu { .. }
            | Mode::SkillMenu { .. }
            | Mode::ItemMenu { .. }
            | Mode::EquipMenu { .. }
            | Mode::Combat(_)
            | Mode::UnitList { .. }
            | Mode::Objective
            | Mode::EndTurnPrompt { .. }
            | Mode::RestartPrompt
            | Mode::SuspendPrompt
            | Mode::Info { .. } => return None,
            Mode::AiAction(a) if !a.shows_cursor() => return None,
            Mode::Selected(sel) if pos == sel.dest() && pos != sel.origin() => return None,
            Mode::Selected(_)
            | Mode::Idle { .. }
            | Mode::MoveAfter { .. }
            | Mode::Targeting(_)
            | Mode::CastTarget(_)
            | Mode::SkillTarget(_)
            | Mode::ItemTarget(_)
            | Mode::TalkTarget { .. }
            | Mode::AiAction(_)
            | Mode::MapMenu { .. } => {}
        }
        Some(CursorView {
            pos,
            brightness: self.cursor.brightness(),
            style: ctx.settings().cursor_style,
        })
    }

    /// Draws the action menu or the weapon list beside its unit, or the
    /// map menu beside the cursor, if open: beside where the map skin has
    /// that tile of `scene`.
    fn draw_menu(&self, ctx: &Ctx, buf: &mut GlyphBuffer, scene: &MapScene) {
        let cells = |tile| ctx.map_skin.tile_cells(scene, MAP_VIEW, tile);
        if let Some(t) = self.forecast() {
            // The arts list, beside the forecast panel (0414).
            if t.has_list() {
                let unit_y = cells(t.sel.dest()).map_or(0, |r| r.y);
                let weapon = (t.sel.unit, &t.with);
                art_list::draw_list(buf, &ctx.palette, &self.state, weapon, &t.list, unit_y);
            }
            return;
        }
        let (tile, menu) = match &self.mode {
            Mode::ActionMenu { sel, menu, .. }
            | Mode::WeaponMenu { sel, menu, .. }
            | Mode::SpellMenu { sel, menu, .. }
            | Mode::SkillMenu { sel, menu, .. }
            | Mode::ItemMenu { sel, menu, .. }
            | Mode::EquipMenu { sel, menu, .. } => (sel.dest(), menu),
            Mode::MapMenu { menu, .. } => (self.cursor.pos, menu),
            _ => return,
        };
        if let Some(tile) = cells(tile) {
            let (x, y) = menu_origin(tile, menu.size());
            menu.draw(&ctx.palette, buf, x, y);
            if matches!(self.mode, Mode::ItemMenu { .. }) {
                // The pack's size in the top border.
                let header = format!(" {} ", items::pack_header(&self.state));
                let (fg, bg) = (
                    ctx.palette.get(UiColor::TextHighlight),
                    ctx.palette.get(UiColor::PanelBg),
                );
                buf.print(x + 2, y, &header, fg, bg);
            }
        }
    }

    /// Ages the heal and burn numbers, the terrain flashes and the message
    /// by `dt` seconds, dropping those whose time is up. A burn number
    /// waits while a banner is on screen: the fire burns out as the phase
    /// banner comes up, which would cover it for longer than it lasts.
    fn tick_popups(&mut self, dt: f32) {
        let banner = self.banner().is_some();
        for p in &mut self.popups {
            if !(banner && p.kind == PopupKind::Burn) {
                p.t += dt;
            }
        }
        self.popups.retain(|p| p.t < TIMINGS.heal_popup);
        for f in &mut self.flashes {
            f.t += dt;
        }
        self.flashes.retain(|f| f.t < TERRAIN_FLASH_S);
        if let Some((_, left)) = &mut self.toast {
            *left -= dt;
            if *left <= 0.0 {
                self.toast = None;
            }
        }
    }

    /// Draws the heal and burn numbers floating up over the units they
    /// tell of: over where the map skin has their tiles of `scene`.
    fn draw_popups(&self, ctx: &Ctx, buf: &mut GlyphBuffer, scene: &MapScene) {
        for p in &self.popups {
            let fg = ctx.palette.get(p.color());
            let Some(tile) = ctx.map_skin.tile_cells(scene, MAP_VIEW, p.pos) else {
                continue;
            };
            let y = (tile.y - 1).max(MAP_VIEW.y);
            let bg = ctx.palette.get(UiColor::Black);
            buf.print(tile.x, y, &p.text(), fg, bg);
        }
    }

    /// Draws the boxes over the map: the unit list, the objective (with
    /// the battle notes under it), the end-turn question, the info screen,
    /// the battle notes at the start, and the banner on screen.
    fn draw_dialogs(&self, ctx: &Ctx, buf: &mut GlyphBuffer, scene: &MapScene) {
        let p = &ctx.palette;
        match &self.mode {
            Mode::UnitList { menu, .. } => map_menu::draw_centred_menu(buf, p, menu),
            Mode::Objective => {
                let mut lines = vec![
                    (map_menu::objective_text(&self.state), UiColor::Text),
                    (map_menu::turn_text(&self.state), UiColor::Text),
                ];
                let noted = notes::note_lines(self.state.battle_notes());
                if !noted.is_empty() {
                    lines.push((String::new(), UiColor::Text));
                    lines.push((notes::NOTES_HEADING.to_owned(), UiColor::TextDim));
                    lines.extend(noted.into_iter().map(|l| (l, UiColor::Text)));
                }
                let (_, h) = map_menu::dialog_size("Objective", &lines);
                let top = self.notes_top(ctx, scene, h);
                map_menu::draw_dialog_at(buf, p, "Objective", &lines, top);
            }
            Mode::EndTurnPrompt { ready } => {
                let km = ctx.help_keys();
                let yes_no = help_line(&[
                    (Some(key_name(km, Action::Confirm)), "yes"),
                    (Some(key_name(km, Action::Cancel)), "no"),
                ])
                .replace(SEPARATOR, " / ");
                let lines = [
                    (map_menu::end_turn_question(*ready), UiColor::Text),
                    (yes_no, UiColor::TextDim),
                ];
                map_menu::draw_dialog(buf, p, "", &lines);
            }
            Mode::RestartPrompt | Mode::SuspendPrompt => {
                let km = ctx.help_keys();
                let yes_no = help_line(&[
                    (Some(key_name(km, Action::Confirm)), "yes"),
                    (Some(key_name(km, Action::Cancel)), "no"),
                ])
                .replace(SEPARATOR, " / ");
                let question = if matches!(self.mode, Mode::SuspendPrompt) {
                    map_menu::SUSPEND_QUESTION
                } else {
                    map_menu::RESTART_QUESTION
                };
                let lines = [
                    (question.to_owned(), UiColor::Text),
                    (yes_no, UiColor::TextDim),
                ];
                map_menu::draw_dialog(buf, p, "", &lines);
            }
            Mode::Info { unit } => {
                if let Some(u) = self.state.unit(*unit) {
                    info::draw_info(buf, p, &self.state, u);
                }
            }
            _ => {}
        }
        if self.notes_open() {
            let lines: Vec<_> = notes::note_lines(self.state.battle_notes())
                .into_iter()
                .map(|l| (l, UiColor::Text))
                .collect();
            let (_, h) = map_menu::dialog_size(notes::NOTES_TITLE, &lines);
            let top = self.notes_top(ctx, scene, h);
            map_menu::draw_dialog_at(buf, p, notes::NOTES_TITLE, &lines, top);
        }
        if let Some(banner) = self.banner() {
            banner.draw(buf, p, map_menu::shown_limit(&self.state));
        }
    }
}

impl BattleScreen {
    /// Draws the rewind screen over its map ([`scene`](Self::scene)): the
    /// list in the side panel and the help line.
    fn draw_rewind(&self, ctx: &Ctx, buf: &mut GlyphBuffer, r: &RewindScreen) {
        r.draw(&ctx.palette, buf);
        let black = ctx.palette.get(UiColor::Black);
        let dim = ctx.palette.get(UiColor::TextDim);
        buf.print(1, HELP_ROW, &self.help(ctx), dim, black);
        draw_debug_hint(ctx, buf, HELP_ROW);
    }

    /// The attack forecast on screen: while an attack's target is picked,
    /// or a spell's with the cursor on an enemy.
    fn forecast(&self) -> Option<&Targeting> {
        match &self.mode {
            Mode::Targeting(t) => Some(t),
            Mode::CastTarget(t) => t.forecast(),
            _ => None,
        }
    }

    /// The line over the help bar: a combat's message, the preview of the
    /// skill, item or spell being aimed, or the toast.
    fn message(&self) -> Option<(String, UiColor)> {
        let preview = match &self.mode {
            Mode::Combat(pb) => pb.message(),
            Mode::SkillTarget(t) => Some(t.preview(&self.state)),
            Mode::ItemTarget(t) => Some(t.preview(&self.state)),
            Mode::CastTarget(t) => t.preview(&self.state),
            _ => None,
        };
        let toast = || Some((self.toast()?.to_owned(), UiColor::TextHighlight));
        preview.map(|text| (text, UiColor::Text)).or_else(toast)
    }

    /// Draws the side panel: the forecast while targeting, else the
    /// terrain and unit under the cursor (as drawn, during a playback).
    fn draw_panel(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        if let Some(t) = self.forecast() {
            forecast::draw_forecast(buf, &ctx.palette, &self.state, t);
            return;
        }
        let panel_bg = c(UiColor::PanelBg);
        buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), panel_bg));
        // Double-line while a unit is selected (`look-and-feel.md`).
        let style = if self.mode.selection().is_some() {
            BoxStyle::Double
        } else {
            BoxStyle::Single
        };
        buf.draw_box(SIDE_PANEL, style, c(UiColor::PanelBorder), panel_bg);
        let pos = self.cursor.pos;
        if self.playing() {
            let shown = self.shown_units();
            let hovered = shown.iter().find(|(u, f)| u.pos == pos && *f < 1.0);
            panel::draw_hover(buf, &ctx.palette, &self.state, pos, hovered.map(|(u, _)| u));
        } else {
            panel::draw_hover(buf, &ctx.palette, &self.state, pos, self.hovered());
        }
    }
}

/// Where a menu of `(w, h)` cells goes beside the tile covering the cells
/// `tile`: one cell right of the tile (clear of the cursor's marks), or
/// left of it if it would run past the map view; its first item level with
/// the tile's top row, moved to fit the view.
fn menu_origin(tile: Rect, (w, h): (i32, i32)) -> (i32, i32) {
    let right = tile.x + tile.w + 1;
    let mx = if right + w <= MAP_VIEW.x + MAP_VIEW.w {
        right
    } else {
        (tile.x - 1 - w).max(MAP_VIEW.x)
    };
    let my = (tile.y - 1)
        .min(MAP_VIEW.y + MAP_VIEW.h - h)
        .max(MAP_VIEW.y);
    (mx, my)
}

/// A view of `size` tiles from `origin` showing `state`'s terrain and
/// nothing else.
fn terrain_scene(state: &BattleState, origin: Pos, size: (i32, i32)) -> MapScene {
    let mut scene = MapScene::new(origin, size);
    let tiles = &state.map().tiles;
    let (w, h) = scene.size;
    for (dx, dy) in (0..h).flat_map(|dy| (0..w).map(move |dx| (dx, dy))) {
        let pos = Pos::new(origin.x + dx, origin.y + dy);
        if let Some(tile) = scene.tile_mut(pos) {
            tile.terrain = tiles.get(pos).copied();
        }
    }
    scene
}

/// The help line on the rewind screen `r`.
fn rewind_help(r: &RewindScreen, ctx: &Ctx) -> String {
    let km = ctx.help_keys();
    let keys = Some(cursor_keys_name(km));
    let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
    let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
    if r.is_confirming() {
        help_line(&[confirm("rewind"), cancel("back")])
    } else if r.can_rewind() {
        help_line(&[(keys, "choose"), confirm("rewind here"), cancel("close")])
    } else if r.entries().is_empty() {
        help_line(&[cancel("close")])
    } else {
        help_line(&[(keys, "choose"), cancel("close")])
    }
}

/// `ON` or `OFF`.
const fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

impl Screen for BattleScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        self.begin_frame(ctx, input.dt);
        let dt = if input.dt.is_finite() {
            input.dt.max(0.0)
        } else {
            0.0
        };
        self.tick_popups(dt);
        self.detect_tips();
        self.tips.absorb(ctx);
        for &action in &input.actions {
            if self.overlay_key(action) {
                continue;
            }
            // The AI's phase: only Confirm (hold: faster) and Cancel (skip
            // a fight) do anything.
            if self.ai_phase() && !matches!(action, Action::Confirm | Action::Cancel) {
                continue;
            }
            if action == Action::ToggleAutoEnd {
                self.toggle_auto_end(ctx);
                continue;
            }
            if self.progress_key(action) {
                continue;
            }
            if self.banner().is_some() {
                if action == Action::Confirm {
                    let t = self.close_banner();
                    if !matches!(t, Transition::None) {
                        return t;
                    }
                }
                continue;
            }
            if self.ai_phase() && !self.playing() {
                continue;
            }
            match action {
                _ if self.rewind.is_some() => self.rewind_step(ctx, action),
                Action::Rewind if self.can_open_rewind() => {
                    let charges = self.state.rewind_charges();
                    self.rewind = Some(RewindScreen::new(&self.history, charges));
                    ctx.audio.menu(MenuSound::Select);
                }
                Action::DangerZone if self.mode.cursor_free() => self.toggle_danger(),
                Action::NextUnit | Action::PrevUnit if matches!(self.mode, Mode::Idle { .. }) => {
                    let from = self.cursor.pos;
                    self.cycle(action == Action::NextUnit);
                    if self.cursor.pos != from {
                        ctx.audio.play_sound(CURSOR_MOVE);
                    }
                }
                Action::CursorLeft
                | Action::CursorRight
                | Action::CursorUp
                | Action::CursorDown
                    if self.mode.cursor_free() =>
                {
                    if self.move_cursor(action) {
                        ctx.audio.play_sound(CURSOR_MOVE);
                    }
                }
                // The map menu's Options: over the battle.
                _ if self.step_mode(ctx, action) => {
                    return Transition::Push(Box::new(OptionsScreen::new()));
                }
                _ => {}
            }
            if self.leaving.is_some() {
                return Transition::Pop;
            }
        }

        self.tick_notes(dt);
        self.animate(ctx, dt, input.is_held(Action::Confirm));
        let waiting = self.shown_tip().is_some() || self.playing();
        if let Some(Queued::Banner(banner)) = self.queue.front_mut()
            && !waiting
        {
            banner.t += dt;
            if banner.expired() {
                // Only phase banners expire, and closing one never leaves.
                self.close_banner();
            }
        }
        if let Some(scene) = self.next_scene(ctx) {
            return Transition::Push(Box::new(DialogueScreen::overlay(
                scene,
                ctx.lead.clone(),
                ctx.content.names.clone(),
            )));
        }
        self.check_auto_end();
        self.drive_ai(ctx);
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        let scene = self.scene(ctx);
        ctx.map_skin.paint(ctx, &scene, MAP_VIEW, buf);
        if let Some(r) = &self.rewind {
            self.draw_rewind(ctx, buf, r);
            return;
        }
        self.draw_menu(ctx, buf, &scene);
        self.draw_popups(ctx, buf, &scene);
        if let Mode::Combat(pb) = &self.mode {
            playback::draw_box(buf, &ctx.palette, pb);
        }
        if let Some(p) = self.progress() {
            progress::draw(buf, &ctx.palette, &ctx.content.portraits, p);
        }
        self.draw_panel(ctx, buf);
        self.draw_dialogs(ctx, buf, &scene);
        if let Some(tip) = self
            .shown_tip()
            .and_then(|t| ctx.content.tips.for_trigger(t))
        {
            let text = fill_placeholders(&tip.text, ctx.help_keys());
            let close = help_line(&[(Some(key_name(ctx.help_keys(), Action::Confirm)), "close")]);
            draw_tip(buf, &ctx.palette, &tip.title, &text, &close);
        }
        buf.fill_rect(HELP_BAR, Cell::new(' ', c(UiColor::Text), black));
        let status = self.status(ctx);
        let w = i32::try_from(status.chars().count()).unwrap_or(0);
        buf.print(
            HELP_BAR.w - 1 - w,
            HELP_BAR.y,
            &status,
            c(UiColor::TextDim),
            black,
        );
        if let Some((message, color)) = self.message() {
            buf.print(1, HELP_BAR.y, &message, c(color), black);
        }
        buf.print(1, HELP_ROW, &self.help(ctx), c(UiColor::TextDim), black);
        draw_debug_hint(ctx, buf, HELP_ROW);
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// Helpers for this module's tests and its submodules'.
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Arc;

    use trpg_core::{
        BattleMap, BattlePack, BattleSetup, BattleState, Command, Objective, Pos, SkillId,
        StatValue, Stock, Unit, UnitAction, UnitId,
    };

    use super::layout::MAP_VIEW;
    use crate::screen::Ctx;

    /// The console cell of the left glyph of `tile` on `s`'s map, if it is
    /// in view.
    pub fn tile_cell(s: &super::BattleScreen, c: &Ctx, tile: Pos) -> Option<(i32, i32)> {
        let cells = c.map_skin.tile_cells(&s.scene(c), MAP_VIEW, tile)?;
        Some((cells.x, cells.y))
    }

    /// A battle on `map` with `units` and `objective`, using the game's
    /// tables, without rewind charges.
    pub fn battle_with(
        c: &Ctx,
        map: BattleMap,
        units: Vec<Unit>,
        objective: Objective,
    ) -> BattleState {
        battle_charged(c, map, units, objective, 0)
    }

    /// [`battle_with`], with `charges` rewind charges.
    pub fn battle_charged(
        c: &Ctx,
        map: BattleMap,
        units: Vec<Unit>,
        objective: Objective,
        charges: u8,
    ) -> BattleState {
        battle_packed(c, map, units, objective, charges, BattlePack::default())
    }

    /// [`battle_charged`], with the battle pack `pack`.
    pub fn battle_packed(
        c: &Ctx,
        map: BattleMap,
        units: Vec<Unit>,
        objective: Objective,
        charges: u8,
        pack: BattlePack,
    ) -> BattleState {
        BattleState::new(BattleSetup {
            pack,
            rewind_charges: charges,
            ..setup(c, map, units, objective)
        })
        .0
    }

    /// The setup of a battle on `map` with `units` and `objective`, using
    /// the game's tables: no pack, rewind charges or triggers, seed 0,
    /// Classic.
    pub fn setup(c: &Ctx, map: BattleMap, units: Vec<Unit>, objective: Objective) -> BattleSetup {
        BattleSetup {
            map,
            terrain: Arc::new(c.content.terrain.rules.clone()),
            classes: Arc::new(c.content.classes.clone()),
            items: Arc::new(c.content.items.clone()),
            spells: Arc::new(c.content.spells.clone()),
            skills: Arc::new(c.content.skills.clone()),
            arts: Arc::new(c.content.arts.clone()),
            pack: BattlePack::default(),
            gold: 0,
            stock: Stock::default(),
            units,
            reinforcements: vec![],
            objective,
            rewind_charges: 0,
            seed: 0,
            triggers: vec![],
            mode: trpg_core::GameMode::Classic,
            battle_notes: vec![],
        }
    }

    /// The Quick Battle's map and units.
    pub fn quick_units(c: &Ctx) -> (BattleMap, Vec<Unit>) {
        let quick = super::quick_battle(&c.content).unwrap_or_else(|e| panic!("{e}"));
        (quick.map().clone(), quick.units().to_vec())
    }

    /// A rout battle on `map` with `units`.
    pub fn battle(c: &Ctx, map: BattleMap, units: Vec<Unit>) -> BattleState {
        battle_with(c, map, units, Objective::Rout { turn_limit: None })
    }

    /// The Quick Battle with the archer (unit 3) at (8, 4), two tiles below
    /// the first brigand (unit 4), knowing Vault, after a Vault attack on
    /// it: the archer waits to move after its attack.
    pub fn vaulted(c: &Ctx) -> BattleState {
        let quick = super::quick_battle(&c.content).unwrap_or_else(|e| panic!("{e}"));
        let mut units = quick.units().to_vec();
        units[2].pos = Pos::new(8, 4);
        assert!(units[2].learn_skill(&SkillId::new("vault"), &c.content.skills));
        let rout = Objective::Rout { turn_limit: None };
        let mut s = battle_with(c, quick.map().clone(), units, rout);
        let attack = Command::Act {
            unit: UnitId(3),
            dest: Pos::new(8, 4),
            action: UnitAction::Attack {
                target: UnitId(4),
                slot: 0,
                active: Some(SkillId::new("vault")),
                art: None,
            },
        };
        if let Err(e) = s.apply(&attack) {
            panic!("{e}");
        }
        assert!(s.pending_move().is_some());
        s
    }

    /// The Quick Battle set for a fight: the lord (unit 1) at (6, 2), one
    /// step left of (7, 2), from where it can hit the raider (unit 6, at
    /// (7, 1)) above and the first brigand (unit 4, at (8, 2), with
    /// `brigand_hp` HP) to its right; the archer (unit 3) at (8, 4), two
    /// tiles below the brigand, which can't counter at that range.
    pub fn skirmish(c: &Ctx, brigand_hp: StatValue) -> BattleState {
        skirmish_charged(c, brigand_hp, 0)
    }

    /// [`skirmish`], with `charges` rewind charges.
    pub fn skirmish_charged(c: &Ctx, brigand_hp: StatValue, charges: u8) -> BattleState {
        let quick = super::quick_battle(&c.content).unwrap_or_else(|e| panic!("{e}"));
        let mut units = quick.units().to_vec();
        units[0].pos = Pos::new(6, 2);
        units[2].pos = Pos::new(8, 4);
        units[3].hp = brigand_hp;
        let rout = Objective::Rout { turn_limit: None };
        battle_charged(c, quick.map().clone(), units, rout, charges)
    }

    /// `unit` of `state` waiting where it stands.
    pub fn wait(state: &mut BattleState, unit: usize) {
        let u = &state.units()[unit];
        let cmd = Command::Act {
            unit: u.id,
            dest: u.pos,
            action: UnitAction::Wait,
        };
        if let Err(e) = state.apply(&cmd) {
            panic!("{e}");
        }
    }

    /// Plays `s` through the AI's phases (0502) in frames of `dt` seconds
    /// with no keys, until the player phase is back or the battle is over.
    /// Returns how many frames that took.
    ///
    /// # Panics
    ///
    /// If that takes more than 1000 frames (e.g. a level-up page waiting
    /// for Confirm).
    pub fn through_ai_phases(s: &mut super::BattleScreen, c: &mut Ctx, dt: f32) -> usize {
        for n in 0..1000 {
            if !s.ai_phase() || s.state().outcome().is_some() {
                return n;
            }
            crate::screen::Screen::update(
                s,
                c,
                &crate::screen::FrameInput::new(vec![], dt, vec![]),
            );
        }
        panic!("the AI phase never ended: {:?}", s.mode());
    }
}

#[cfg(test)]
mod art_tests;
#[cfg(test)]
mod attack_tests;

#[cfg(test)]
mod item_tests;

#[cfg(test)]
mod magic_tests;

#[cfg(test)]
mod progress_tests;

#[cfg(test)]
mod rewind_tests;

#[cfg(test)]
mod scene_tests;
#[cfg(test)]
mod settings_tests;

#[cfg(test)]
mod skill_tests;

#[cfg(test)]
mod sound_tests;

#[cfg(test)]
mod tip_tests;

#[cfg(test)]
mod turn_tests;

#[cfg(test)]
mod ai_phase_tests;

#[cfg(test)]
mod trigger_tests;

#[cfg(test)]
mod notes_tests;

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use trpg_core::{BattleMap, Grid, Objective, Phase, TerrainId, Unit};

    use super::testing::{battle, vaulted, wait};
    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::harness::Harness;
    use crate::map_view::CursorStyle;
    use crate::screen::tests::ctx;

    fn quick() -> BattleScreen {
        BattleScreen::new(quick_battle(&ctx().content).unwrap())
    }

    fn render(screen: &BattleScreen, c: &Ctx) -> GlyphBuffer {
        let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
        screen.draw(c, &mut buf);
        buf
    }

    #[test]
    fn quick_battle_state() {
        let state = quick_battle(&ctx().content).unwrap();
        assert_eq!(state.map().name, "Test Field");
        assert_eq!((state.turn(), state.phase()), (1, Phase::Player));
        assert_eq!(state.objective(), Objective::Rout { turn_limit: None });
        assert_eq!(state.outcome(), None);
        let units = state.units();
        let labels: Vec<(&str, Faction)> = units
            .iter()
            .map(|u| (u.map_label.as_str(), u.faction))
            .collect();
        assert_eq!(
            labels,
            [
                ("Lo", Faction::Player),
                ("Kn", Faction::Player),
                ("Ar", Faction::Player),
                ("Br", Faction::Enemy),
                ("Br", Faction::Enemy),
                ("Ra", Faction::Enemy),
                ("Fr", Faction::Enemy),
                ("Ma", Faction::Player),
            ]
        );
        let ids: Vec<u32> = units.iter().map(|u| u.id.0).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, 6, 7, 8]);
        for u in units {
            assert!(!u.acted, "{}", u.name);
            assert_eq!(u.hp, u.stats.hp, "{}", u.name);
            assert!(state.map().tiles.in_bounds(u.pos));
        }
        // The caster (0410), numbered after the enemies: Fire from its
        // class, Frost and Heal of its own, all at full uses, Fire equipped.
        let mage = state.unit(UnitId(8)).unwrap();
        assert_eq!(
            (mage.name.as_str(), mage.pos),
            ("Test Mage", Pos::new(3, 6))
        );
        let spells: Vec<(&str, u8)> = mage
            .learned
            .iter()
            .map(|s| (s.0.as_str(), mage.spells.uses_left(s)))
            .collect();
        assert_eq!(spells, [("fire", 10), ("frost", 10), ("heal", 8)]);
        let fire = trpg_core::SpellId::new("fire");
        assert_eq!(mage.loadout.equipped, Some(Equipped::Spell(fire)));
        // The elemental holds its tile.
        let elemental = state.unit(UnitId(7)).unwrap();
        assert_eq!(elemental.class.0, "frost_elemental");
        assert_eq!(elemental.ai, trpg_core::AiBehavior::Stationary);
        assert_eq!(elemental.pos, Pos::new(8, 7));
    }

    #[test]
    fn quick_battle_reports_missing_content() {
        let mut content = ctx().content;
        content.battles.clear();
        assert_eq!(
            quick_battle(&content),
            Err("no battle \"quick\"".to_owned())
        );
    }

    #[test]
    fn camera_starts_on_the_first_player_unit() {
        let c = ctx();
        let s = quick();
        // test_small is smaller than the viewport: centred.
        assert_eq!(s.camera().origin, Pos::new(-10, -11));
        assert_eq!(s.state().units().len(), 8);
        let big = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        let units = s.state().units().to_vec();
        let camera = |units: Vec<Unit>| {
            BattleScreen::new(battle(&c, big.clone(), units))
                .camera()
                .origin
        };
        // Lord at (3, 5): clamped to the top-left.
        assert_eq!(camera(units.clone()), Pos::new(0, 0));
        let mut far = units.clone();
        far[0].pos = Pos::new(60, 30);
        assert_eq!(camera(far), Pos::new(29, 10));
        let mut enemies_only = units;
        enemies_only.retain(|u| u.faction != Faction::Player);
        // Map centre (32, 20).
        assert_eq!(camera(enemies_only), Pos::new(15, 5));
    }

    #[test]
    fn cancel_opens_the_map_menu_and_nothing_pops() {
        let mut s = quick();
        let mut c = ctx();
        let step = |s: &mut BattleScreen, c: &mut Ctx, a: &[Action]| {
            format!(
                "{:?}",
                s.update(c, &FrameInput::new(a.to_vec(), 0.0, vec![]))
            )
        };
        assert_eq!(s.name(), "battle");
        assert!(!s.is_overlay());
        assert_eq!(step(&mut s, &mut c, &[]), "None");
        // Select the lord, then cancel the selection: still here.
        assert_eq!(step(&mut s, &mut c, &[Action::Confirm]), "None");
        assert_eq!(step(&mut s, &mut c, &[Action::Cancel]), "None");
        assert_eq!(s.mode(), &Mode::default());
        // Browsing: Cancel opens the map menu, and Cancel again closes it.
        assert_eq!(step(&mut s, &mut c, &[Action::Cancel]), "None");
        assert!(matches!(s.mode(), Mode::MapMenu { .. }), "{:?}", s.mode());
        assert_eq!(step(&mut s, &mut c, &[Action::Cancel]), "None");
        assert_eq!(s.mode(), &Mode::default());
    }

    #[test]
    fn help_depends_on_what_is_hovered() {
        let mut c = ctx();
        let mut state = quick_battle(&c.content).unwrap();
        // The archer waits where it stands.
        wait(&mut state, 2);
        let mut s = BattleScreen::new(state);
        // On the lord, ready to act.
        assert_eq!(
            s.help(&c),
            "f select · e info · s next unit · r rewind · d menu · Space end turn"
        );
        // On the archer, who has acted, and on an enemy.
        s.cursor.jump(Pos::new(2, 4));
        assert_eq!(
            s.help(&c),
            "arrows move · e info · s next unit · r rewind · d menu · Space end turn"
        );
        s.cursor.jump(Pos::new(8, 2));
        assert_eq!(
            s.help(&c),
            "arrows move · f range · e info · s next unit · r rewind · d menu · Space end turn"
        );
        // An enemy's range shown: Cancel hides it.
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(
            s.help(&c),
            "arrows move · f range · e info · s next unit · r rewind · d hide range · Space end turn"
        );
        step(&mut s, &mut c, &[Action::Cancel]);
        // On an empty tile.
        s.cursor.jump(Pos::new(0, 0));
        assert_eq!(
            s.help(&c),
            "arrows move · f menu · s next unit · r rewind · d menu · Space end turn"
        );
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(
            s.help(&c),
            "wasd move · j menu · l next unit · u rewind · k menu · Space end turn"
        );
        s.cursor.jump(Pos::new(3, 5));
        assert_eq!(
            s.help(&c),
            "j select · i info · l next unit · u rewind · k menu · Space end turn"
        );
    }

    #[test]
    fn help_shows_not_mapped_for_an_action_with_no_key() {
        let mut c = ctx();
        let s = quick();
        let mut keys = c.layout_bindings(crate::input::Layout::RightHanded);
        keys.clear(Action::Info, 0);
        c.set_layout_bindings(crate::input::Layout::RightHanded, keys)
            .unwrap();
        assert_eq!(
            s.help(&c),
            "f select · ! not mapped info · s next unit · r rewind · d menu · Space end turn"
        );
    }

    #[test]
    fn cursor_starts_on_the_first_player_lord() {
        let c = ctx();
        let s = quick();
        assert_eq!(s.cursor(), Cursor::new(Pos::new(3, 5)));
        // The lord listed after another player unit: still the lord.
        let mut units = s.state().units().to_vec();
        units.swap(0, 1);
        let map = s.state().map().clone();
        let s2 = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        assert_eq!(s2.cursor().pos, Pos::new(3, 5));
        // No lord: the first player unit.
        for u in &mut units {
            u.is_lord = false;
        }
        let s3 = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        assert_eq!(s3.cursor().pos, Pos::new(4, 6));
        // No player unit: the map's centre (test_small is 14 × 8).
        units.retain(|u| u.faction != Faction::Player);
        let s4 = BattleScreen::new(battle(&c, map, units));
        assert_eq!(s4.cursor().pos, Pos::new(7, 4));
    }

    /// One frame with `actions`.
    fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
        s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
    }

    #[test]
    fn cursor_moves_one_tile_per_step_and_stays_on_the_map() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::CursorRight; 3]);
        assert_eq!(s.cursor().pos, Pos::new(6, 5));
        step(&mut s, &mut c, &[Action::CursorDown, Action::CursorLeft]);
        assert_eq!(s.cursor().pos, Pos::new(5, 6));
        step(&mut s, &mut c, &[Action::CursorUp; 20]);
        assert_eq!(s.cursor().pos, Pos::new(5, 0));
        step(&mut s, &mut c, &[Action::CursorLeft; 20]);
        assert_eq!(s.cursor().pos, Pos::new(0, 0));
        step(&mut s, &mut c, &[Action::CursorDown; 20]);
        step(&mut s, &mut c, &[Action::CursorRight; 20]);
        assert_eq!(s.cursor().pos, Pos::new(13, 7));
        // Other keys don't move it; the small map stays centred.
        step(
            &mut s,
            &mut c,
            &[Action::Confirm, Action::Info, Action::DangerZone],
        );
        assert_eq!(s.cursor().pos, Pos::new(13, 7));
        assert_eq!(s.camera().origin, Pos::new(-10, -11));
    }

    #[test]
    fn cursor_pulses_with_frame_time() {
        let mut c = ctx();
        let mut s = quick();
        s.update(&mut c, &FrameInput::new(vec![], 0.5, vec![]));
        assert!((s.cursor().brightness() - cursor::BLINK_MIN).abs() < 1e-6);
        // The map shows it on the lord at (3, 5), dimmed; a fresh battle's
        // at full brightness.
        let shown = |s: &BattleScreen| s.scene(&c).cursor.map(|v| (v.pos, v.brightness));
        assert_eq!(shown(&s), Some((Pos::new(3, 5), cursor::BLINK_MIN)));
        assert_eq!(shown(&quick()), Some((Pos::new(3, 5), 1.0)));
    }

    #[test]
    fn next_and_prev_unit_cycle_ready_units_in_reading_order() {
        let mut c = ctx();
        let mut s = quick();
        // Ready, in reading order: the archer (2, 4), the lord (3, 5), the
        // mage (3, 6) and the knight (4, 6).
        assert_eq!(
            s.ready_units(),
            [
                Pos::new(2, 4),
                Pos::new(3, 5),
                Pos::new(3, 6),
                Pos::new(4, 6)
            ]
        );
        let mut visit = |a: Action| {
            step(&mut s, &mut c, &[a]);
            s.cursor().pos
        };
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 6));
        assert_eq!(visit(Action::NextUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::NextUnit), Pos::new(2, 4));
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 5));
        assert_eq!(visit(Action::PrevUnit), Pos::new(2, 4));
        assert_eq!(visit(Action::PrevUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::PrevUnit), Pos::new(3, 6));
        assert_eq!(visit(Action::PrevUnit), Pos::new(3, 5));
        // From a tile between them, in reading order.
        assert_eq!(visit(Action::CursorRight), Pos::new(4, 5));
        assert_eq!(visit(Action::NextUnit), Pos::new(3, 6));
        assert_eq!(visit(Action::NextUnit), Pos::new(4, 6));
        assert_eq!(visit(Action::CursorUp), Pos::new(4, 5));
        assert_eq!(visit(Action::PrevUnit), Pos::new(3, 5));
        // Below every ready unit: next wraps to the first.
        for _ in 0..5 {
            visit(Action::CursorDown);
        }
        assert_eq!(visit(Action::NextUnit), Pos::new(2, 4));
    }

    #[test]
    fn cycling_does_nothing_without_ready_units_and_scrolls_the_camera() {
        let c = ctx();
        let mut ctx_ = ctx();
        let mut units = quick_battle(&c.content).unwrap().units().to_vec();
        for u in &mut units {
            u.acted = u.faction == Faction::Player;
        }
        let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        let mut s = BattleScreen::new(battle(&c, map.clone(), units.clone()));
        step(&mut s, &mut ctx_, &[Action::NextUnit, Action::PrevUnit]);
        assert_eq!(s.cursor().pos, Pos::new(3, 5));
        // A ready unit far away: the camera follows the jump.
        units[1].acted = false;
        units[1].pos = Pos::new(60, 35);
        let mut s = BattleScreen::new(battle(&c, map, units));
        // (Past the mage, the one unit between the lord and it.)
        step(&mut s, &mut ctx_, &[Action::NextUnit, Action::NextUnit]);
        assert_eq!(s.cursor().pos, Pos::new(60, 35));
        // Scrolled just enough to keep it 3 tiles from the edges.
        assert_eq!(s.camera().origin, Pos::new(29, 9));
    }

    /// The Quick Battle's units on a 64 × 40 plain, in the harness.
    fn big_battle_harness() -> Harness {
        let c = ctx();
        let units = quick_battle(&c.content).unwrap().units().to_vec();
        let map = BattleMap::new("Big", Grid::filled(64, 40, TerrainId(0)));
        Harness::with_screen(Box::new(BattleScreen::new(battle(&c, map, units))))
    }

    /// Where the cursor is in the map view, as tiles right of and below
    /// its top-left tile.
    fn cursor_in_view(h: &Harness) -> (i32, i32) {
        let scene = h.map_scene().expect("no battle");
        let pos = scene.cursor_tile().expect("no cursor on the map");
        scene.offset(pos).expect("the cursor is in the view")
    }

    /// Holding a cursor key ticks once per tile moved, and not at the
    /// map's edge (ticket 0425).
    #[test]
    fn held_cursor_ticks_once_per_tile() {
        let ticks = |h: &Harness| h.sounds().iter().filter(|c| *c == CURSOR_MOVE).count();
        let mut h = big_battle_harness();
        // The lord at (3, 5); the camera scrolls as the cursor goes right.
        h.hold("Right", 1.0);
        let right = ticks(&h);
        assert!(right > 5, "{right} ticks: the key should repeat");
        assert_eq!(h.sounds().len(), right, "{:?}", h.sounds());
        // Held left for much longer than it takes to reach x = 0: one tick
        // per tile back, none once at the edge.
        h.clear_audio().hold("Left", 6.0);
        assert_eq!(ticks(&h), right + 3);
        assert_eq!(cursor_in_view(&h).0, 0, "at the left edge");
        h.clear_audio().keys("Left Up Up Up Up Up Up");
        assert_eq!(ticks(&h), 5, "only the five steps up to y = 0");
        // Next unit jumps: one tick.
        h.clear_audio().keys("s");
        assert_eq!(h.sounds(), [CURSOR_MOVE]);
    }

    #[test]
    fn harness_keys_move_the_cursor() {
        let mut h = big_battle_harness();
        // The lord at (3, 5), camera at the top-left.
        assert_eq!(cursor_in_view(&h), (3, 5));
        h.keys("Right Right Right");
        assert_eq!(cursor_in_view(&h), (6, 5));
        h.keys("Down Left");
        assert_eq!(cursor_in_view(&h), (5, 6));
    }

    #[test]
    fn harness_hold_repeats_with_the_keymap_timing_and_scrolls() {
        let c = ctx();
        let repeat = c.content.keymap.repeat;
        let (delay, interval) = (repeat.delay_ms, repeat.interval_ms);
        let moves = |ms: u32| i32::try_from(1 + 1 + (ms - delay) / interval).unwrap();
        let mut h = big_battle_harness();
        h.hold("Right", 1.2);
        // 1 press + repeats at 300 ms, then every 55 ms: 18 tiles, to x = 21.
        assert_eq!(moves(1200), 18);
        assert_eq!(cursor_in_view(&h), (3 + 18, 5));
        h.hold("Right", 1.2);
        // x = 39: the camera keeps it 3 tiles from the right edge.
        let x = 3 + 2 * moves(1200);
        let view_w = h.map_scene().unwrap().size.0;
        assert_eq!(view_w, 35);
        let origin = x - (view_w - 1 - Camera::MARGIN);
        assert_eq!(cursor_in_view(&h), (x - origin, 5));
        // Far right, then back: the cursor stops at the edge and the
        // camera shows the map's last columns, then scrolls back.
        h.hold("Right", 3.0).hold("Down", 3.0);
        assert_eq!(cursor_in_view(&h), (34, 29));
        h.hold("Left", 4.0).hold("Up", 3.0);
        assert_eq!(cursor_in_view(&h), (0, 0));
    }

    /// The text of row `y` of the side panel, inside its border, trimmed.
    fn panel_row(buf: &GlyphBuffer, y: i32) -> String {
        let (x0, x1) = (SIDE_PANEL.x + 1, SIDE_PANEL.x + SIDE_PANEL.w - 1);
        (x0..x1)
            .map(|x| buf.get(x, y).unwrap().glyph)
            .collect::<String>()
            .trim()
            .to_owned()
    }

    /// The Quick Battle with the knight wounded (9/20 HP) and moved into
    /// the forest at (1, 5).
    fn knight_in_forest() -> BattleScreen {
        let c = ctx();
        let state = quick_battle(&c.content).unwrap();
        let mut units = state.units().to_vec();
        units[1].pos = Pos::new(1, 5);
        units[1].hp = units[1].stats.hp * 9 / 20;
        let mut s = BattleScreen::new(battle(&c, state.map().clone(), units));
        s.cursor.jump(Pos::new(1, 5));
        s
    }

    #[test]
    fn panel_shows_the_terrain_and_unit_under_the_cursor() {
        let c = ctx();
        let s = knight_in_forest();
        let knight = s.hovered().unwrap().clone();
        let buf = render(&s, &c);
        let rows: Vec<String> = (1..10).map(|y| panel_row(&buf, y)).collect();
        let hp = format!("HP {}/{}", knight.hp, knight.stats.hp);
        assert_eq!(rows[0], "Forest");
        assert_eq!(rows[1], "DEF +1  AVO +20");
        assert_eq!(rows[2], "");
        assert_eq!(rows[4], "Test Knight");
        assert_eq!(rows[5], "Guard  Lv 1");
        assert!(rows[6].starts_with(&hp), "{}", rows[6]);
        assert_eq!(rows[7], "Player");
        // The knight is at 9/20 of max HP: 5 of 10 cells, `hp_mid`.
        let bar: String = (0..panel::HP_BAR_CELLS)
            .map(|i| buf.get(panel::HP_BAR_X + i, 7).unwrap().glyph)
            .collect();
        assert_eq!(bar, "█████░░░░░");
        let p = &c.palette;
        assert_eq!(
            buf.get(panel::HP_BAR_X, 7).unwrap().fg,
            p.get(UiColor::HpMid)
        );
        assert_eq!(
            buf.get(panel::TEXT_X, 5).unwrap().fg,
            p.get(UiColor::Player)
        );
        // An enemy: its faction, in red.
        let mut s = s;
        s.cursor.jump(Pos::new(8, 2));
        let buf = render(&s, &c);
        assert_eq!(panel_row(&buf, 8), "Enemy");
        assert_eq!(buf.get(panel::TEXT_X, 5).unwrap().fg, p.get(UiColor::Enemy));
    }

    #[test]
    fn panel_shows_healing_terrain_and_cuts_long_names() {
        let mut c = ctx();
        let s = knight_in_forest();
        let mut rules = c.content.terrain.rules.clone();
        let forest = c.content.terrain.display.id_of("forest").unwrap();
        rules.terrains[usize::from(forest.0)].heal_percent = 20;
        rules.terrains[usize::from(forest.0)].name = "A".repeat(40);
        c.content.terrain.rules = rules;
        let mut units = s.state().units().to_vec();
        units[1].name = "B".repeat(40);
        let mut s2 = BattleScreen::new(battle(&c, s.state().map().clone(), units));
        s2.cursor.jump(Pos::new(1, 5));
        let buf = render(&s2, &c);
        assert_eq!(panel_row(&buf, 1), "A".repeat(panel::TEXT_W));
        assert_eq!(panel_row(&buf, 3), "Heals 20% HP");
        assert_eq!(panel_row(&buf, 5), "B".repeat(panel::TEXT_W));
    }

    /// Hovering the wounded knight, standing in a forest.
    #[test]
    fn hover_unit_on_forest_snapshot() {
        let c = ctx();
        assert_snapshot!(render(&knight_in_forest(), &c).to_snapshot(&c.palette));
    }

    /// Hovering an empty plain: terrain only.
    #[test]
    fn hover_empty_plain_snapshot() {
        let c = ctx();
        let mut s = quick();
        s.cursor.jump(Pos::new(6, 5));
        let buf = render(&s, &c);
        assert_eq!(panel_row(&buf, 1), "Plain");
        assert_eq!(panel_row(&buf, 5), "");
        assert_snapshot!(buf.to_snapshot(&c.palette));
    }

    #[test]
    fn draw_covers_the_whole_buffer() {
        let c = ctx();
        let buf = render(&quick(), &c);
        let stale = Rgb::new(1, 2, 3);
        for y in 0..i32::from(CONSOLE_H) {
            for x in 0..i32::from(CONSOLE_W) {
                let cell = buf.get(x, y).unwrap();
                assert!(cell.fg != stale && cell.bg != stale, "({x}, {y})");
            }
        }
    }

    #[test]
    fn terrain_tiles_use_their_two_glyphs_and_colours() {
        let c = ctx();
        let s = quick();
        let buf = render(&s, &c);
        let p = &c.palette;
        // Tile (0, 0) is sea, drawn at cell (20, 11).
        let display = &c.content.terrain.display;
        let sea = display.get(display.id_of("sea").unwrap()).unwrap();
        let cell = |x| *buf.get(x, 11).unwrap();
        for x in [20, 21] {
            assert_eq!(cell(x).glyph, sea.glyphs[usize::try_from(x - 20).unwrap()]);
            assert_eq!(Some(cell(x).fg), p.lookup(&sea.fg));
            assert_eq!(Some(cell(x).bg), p.lookup(&sea.bg));
        }
        // Left of the map: blank.
        assert_eq!(
            cell(19),
            Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black))
        );
    }

    /// A 64 × 40 map of stripes, a unit in the bottom-right corner and the
    /// cursor and camera moved there.
    #[test]
    fn large_map_scrolled_to_bottom_right_snapshot() {
        let c = ctx();
        let display = &c.content.terrain.display;
        let ids =
            ["plain", "forest", "road", "water", "mountain"].map(|t| display.id_of(t).unwrap());
        let cells = (0..40)
            .flat_map(|y: usize| (0..64).map(move |x: usize| ids[(x / 3 + y / 2) % ids.len()]))
            .collect();
        let mut units = quick_battle(&c.content).unwrap().units().to_vec();
        // The camera starts on the lord at (3, 5), top-left.
        units[5].pos = Pos::new(63, 39);
        let cornered = units[5].id;
        units[3].pos = Pos::new(30, 12);
        units[4].pos = Pos::new(28, 9); // Above the viewport: not drawn.
        let map = BattleMap::new("Stripes", Grid::from_cells(64, 40, cells).unwrap());
        let mut s = BattleScreen::new(battle(&c, map, units));
        assert_eq!(s.camera().origin, Pos::new(0, 0));
        s.cursor.jump(Pos::new(63, 39));
        s.follow(Pos::new(63, 39));
        assert_eq!(s.camera().origin, Pos::new(29, 10));
        // The corner unit sits in the viewport's last tile.
        let scene = s.scene(&c);
        let corner = Pos::new(63, 39);
        assert_eq!(scene.size, (35, 30));
        assert_eq!(scene.offset(corner), Some((34, 29)));
        assert_eq!(scene.unit_at(corner).map(|u| u.id), Some(cornered));
        assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
    }

    /// One frame of `dt` seconds with `actions`, Confirm held if `held`.
    fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32, held: bool) {
        let held = if held { vec![Action::Confirm] } else { vec![] };
        s.update(c, &FrameInput::new(actions.to_vec(), dt, held));
    }

    /// The Quick Battle with the lord selected and walked two tiles right to
    /// (5, 5), its action menu open.
    fn lord_menu(c: &mut Ctx) -> BattleScreen {
        let mut s = quick();
        step(
            &mut s,
            c,
            &[Action::Confirm, Action::CursorRight, Action::CursorRight],
        );
        step(&mut s, c, &[Action::Confirm]);
        assert!(matches!(s.mode(), Mode::Moving { .. }), "{:?}", s.mode());
        frame(&mut s, c, &[], 1.0, false);
        assert!(
            matches!(s.mode(), Mode::ActionMenu { .. }),
            "{:?}",
            s.mode()
        );
        s
    }

    #[test]
    fn select_move_and_wait_ends_the_units_action() {
        let mut c = ctx();
        let mut s = lord_menu(&mut c);
        step(&mut s, &mut c, &[Action::Confirm]);
        let lord = &s.state().units()[0];
        assert_eq!((lord.pos, lord.acted), (Pos::new(5, 5), true));
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.cursor().pos, Pos::new(5, 5));
        // No longer ready: not selectable, and cycling skips it.
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(
            s.ready_units(),
            [Pos::new(2, 4), Pos::new(3, 6), Pos::new(4, 6)]
        );
    }

    #[test]
    fn cancelling_from_the_menu_and_the_selection_leaves_the_battle_unchanged() {
        let mut c = ctx();
        let before = quick().state().clone();
        let mut s = lord_menu(&mut c);
        // The lord is drawn at (5, 5), but the battle hasn't changed.
        assert_eq!(s.hovered().map(|u| u.id), Some(UnitId(1)));
        assert_eq!(s.state(), &before);
        step(&mut s, &mut c, &[Action::Cancel]);
        let Mode::Selected(sel) = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(sel.path, [Pos::new(3, 5), Pos::new(4, 5), Pos::new(5, 5)]);
        assert_eq!(s.hovered(), None, "drawn back at its own tile");
        assert_eq!(s.state(), &before);
        step(&mut s, &mut c, &[Action::Cancel]);
        assert_eq!(s.mode(), &Mode::default());
        assert_eq!(s.cursor().pos, Pos::new(3, 5));
        assert_eq!(s.state(), &before);
    }

    #[test]
    fn a_held_confirm_skips_the_walk() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm]);
        step(&mut s, &mut c, &[Action::CursorRight; 2]);
        // The press starts the walk; still held after 0.2 s, it skips.
        frame(&mut s, &mut c, &[Action::Confirm], 0.0, true);
        frame(&mut s, &mut c, &[], mode::HOLD_SKIP_S / 2.0, true);
        assert!(matches!(s.mode(), Mode::Moving { .. }));
        frame(&mut s, &mut c, &[], mode::HOLD_SKIP_S / 2.0, true);
        assert!(matches!(s.mode(), Mode::ActionMenu { .. }));
    }

    #[test]
    fn keys_do_only_what_the_mode_allows() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
        // Selected: cycling does nothing.
        step(&mut s, &mut c, &[Action::NextUnit, Action::PrevUnit]);
        assert_eq!(s.cursor().pos, Pos::new(4, 5));
        // In the menu, cursor keys move its focus, not the cursor.
        let mut s = lord_menu(&mut c);
        step(
            &mut s,
            &mut c,
            &[Action::CursorUp, Action::CursorLeft, Action::NextUnit],
        );
        assert_eq!(s.cursor().pos, Pos::new(5, 5));
        // While walking, too.
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
        step(&mut s, &mut c, &[Action::Confirm, Action::CursorDown]);
        assert_eq!(s.cursor().pos, Pos::new(4, 5));
    }

    #[test]
    fn selecting_draws_ranges_and_a_double_panel_border() {
        let mut c = ctx();
        let mut s = quick();
        let plain = render(&s, &c);
        assert!(s.scene(&c).tints_at(Pos::new(6, 5)).is_empty());
        step(&mut s, &mut c, &[Action::Confirm]);
        let buf = render(&s, &c);
        let scene = s.scene(&c);
        // The cursor stays on the lord, as corner marks.
        let cursor = scene.cursor.unwrap();
        let lord = Pos::new(3, 5);
        assert_eq!((cursor.pos, cursor.style), (lord, CursorStyle::Corners));
        // (6, 5), reachable, is in the move range.
        assert_eq!(scene.tints_at(Pos::new(6, 5)), [RangeKind::Move]);
        let Mode::Selected(sel) = s.mode() else {
            panic!()
        };
        let attack = sel.attack.iter().next().unwrap();
        assert_eq!(scene.tints_at(attack), [RangeKind::Attack]);
        assert_eq!(buf.get(SIDE_PANEL.x, 0).unwrap().glyph, '╔');
        assert_eq!(plain.get(SIDE_PANEL.x, 0).unwrap().glyph, '┌');
        // No path yet: only the lord's own tile.
        assert_eq!(scene.path, [lord]);
    }

    #[test]
    fn the_path_ends_in_an_arrowhead_with_no_cursor_frame_there() {
        let mut c = ctx();
        let mut s = quick();
        step(
            &mut s,
            &mut c,
            &[Action::Confirm, Action::CursorRight, Action::CursorRight],
        );
        // The path from the lord ends on the fort at (5, 5) (each skin
        // draws its end its own way: the glyph skin an arrowhead): no
        // cursor there, and the lord still on its tile.
        let scene = s.scene(&c);
        let (lord, fort) = (Pos::new(3, 5), Pos::new(5, 5));
        assert_eq!(scene.path, [lord, Pos::new(4, 5), fort]);
        assert_eq!(scene.cursor, None);
        assert_eq!(scene.unit_at(lord).map(|u| u.label.as_str()), Some("Lo"));
        assert_eq!(scene.unit_at(fort), None);
        let fort_id = c.content.terrain.display.id_of("fort");
        assert_eq!(scene.terrain_at(fort), fort_id);
        // On to the map's right edge, past the lord's reach (Mov 5): the
        // path, through the fort (cost 2), stops at (7, 5), and the cursor
        // shows its corner marks.
        step(&mut s, &mut c, &[Action::CursorRight; 8]);
        assert_eq!(s.cursor().pos, Pos::new(13, 5));
        let Mode::Selected(sel) = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(sel.dest(), Pos::new(7, 5));
        assert_eq!(s.scene(&c).cursor_tile(), Some(Pos::new(13, 5)));
    }

    #[test]
    fn the_menu_opens_beside_the_unit_drawn_at_its_new_tile() {
        let mut c = ctx();
        let s = lord_menu(&mut c);
        let buf = render(&s, &c);
        let row = |y: i32, from: i32, n: i32| -> String {
            (from..from + n)
                .map(|x| buf.get(x, y).unwrap().glyph)
                .collect()
        };
        // The lord at (5, 5); its old tile is empty.
        let scene = s.scene(&c);
        let lord = scene.unit_at(Pos::new(5, 5));
        assert_eq!(lord.map(|u| u.label.as_str()), Some("Lo"));
        assert_eq!(scene.unit_at(Pos::new(3, 5)), None);
        // The menu one cell right of the tile: `Attack` and `Item` (dim),
        // `Skill`, `Equip`, `Wait`.
        assert_eq!(row(16, 33, 10), "│ Attack │");
        assert_eq!(row(17, 33, 10), "│ Skill  │");
        assert_eq!(row(18, 33, 10), "│ Item   │");
        assert_eq!(row(19, 33, 10), "│ Equip  │");
        assert_eq!(row(20, 33, 10), "│ Wait   │");
        let p = &c.palette;
        assert_eq!(buf.get(35, 16).unwrap().fg, p.get(UiColor::TextDim));
        assert_eq!(buf.get(35, 18).unwrap().fg, p.get(UiColor::TextDim));
        assert_eq!(
            buf.get(35, 20).unwrap().bg,
            p.get(UiColor::PanelBorderFocus)
        );
        // No cursor, no ranges, the panel shows the lord.
        assert_eq!(scene.cursor, None);
        assert!(scene.tiles.iter().all(|t| t.tints.is_empty()));
        assert_eq!(panel_row(&buf, 5), "Test Lord");
    }

    #[test]
    fn menus_go_right_of_the_tile_unless_they_would_leave_the_view() {
        // A glyph tile: two cells wide, one high.
        let beside = |(x, y), size| menu_origin(Rect::new(x, y, 2, 1), size);
        // The first item (under the top border) level with the tile.
        assert_eq!(beside((10, 4), (10, 4)), (13, 3));
        // Too far right: left of the tile.
        assert_eq!(beside((60, 4), (10, 4)), (49, 3));
        assert_eq!(beside((57, 4), (10, 4)), (60, 3));
        // Too low or high: moved to fit; never off the left edge.
        assert_eq!(beside((10, 29), (10, 4)), (13, 26));
        assert_eq!(beside((2, 0), (70, 4)), (0, 0));
        // A bigger tile (another skin's): right of all of it, level with
        // its top row.
        assert_eq!(menu_origin(Rect::new(10, 4, 4, 2), (10, 4)), (15, 3));
        assert_eq!(menu_origin(Rect::new(58, 4, 4, 2), (10, 4)), (47, 3));
    }

    #[test]
    fn help_follows_the_mode() {
        let mut c = ctx();
        let mut s = quick();
        step(&mut s, &mut c, &[Action::Confirm]);
        // On the unit: its own tile is a legal end.
        assert_eq!(s.help(&c), "arrows move · f move here · d cancel");
        step(&mut s, &mut c, &[Action::CursorRight, Action::CursorDown]);
        // On the knight: can't stop there.
        assert_eq!(s.help(&c), "arrows move · d cancel");
        step(&mut s, &mut c, &[Action::CursorUp, Action::CursorRight]);
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.help(&c), "f skip");
        frame(&mut s, &mut c, &[], 1.0, false);
        assert_eq!(s.help(&c), "arrows choose · f confirm · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(s.help(&c), "wasd choose · j confirm · k back");
    }

    #[test]
    fn an_enemys_threat_area_is_tinted_until_hidden() {
        let mut c = ctx();
        let mut s = quick();
        let brigand = Pos::new(8, 2);
        s.cursor.jump(brigand);
        assert!(s.scene(&c).tints_at(brigand).is_empty());
        step(&mut s, &mut c, &[Action::Confirm]);
        // The brigand's tile and one it can reach.
        let shown = s.scene(&c);
        for pos in [brigand, Pos::new(8, 7)] {
            assert_eq!(shown.tints_at(pos), [RangeKind::Attack], "{pos:?}");
        }
        step(&mut s, &mut c, &[Action::Confirm]);
        assert!(s.scene(&c).tints_at(brigand).is_empty());
    }

    #[test]
    fn a_pending_move_after_an_attack_is_chosen_on_the_map() {
        let mut c = ctx();
        let state = vaulted(&c);
        let tiles = state.move_after_tiles();
        let mut s = BattleScreen::new(state);
        let Mode::MoveAfter { unit, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        assert_eq!(*unit, UnitId(3));
        // Its tiles are in a move range.
        assert_eq!(s.scene(&c).tints_at(tiles[0]), [RangeKind::Move]);
        assert!(quick().scene(&c).tints_at(tiles[0]).is_empty());
        s.cursor.jump(Pos::new(8, 4));
        assert_eq!(s.help(&c), "arrows move · f stay");
        s.cursor.jump(tiles[0]);
        assert_eq!(s.help(&c), "arrows move · f move here");
        s.cursor.jump(Pos::new(0, 0));
        assert_eq!(s.help(&c), "arrows move");
        // Cancel doesn't leave; Confirm on a tile moves there.
        step(&mut s, &mut c, &[Action::Cancel]);
        assert!(matches!(s.mode(), Mode::MoveAfter { .. }));
        s.cursor.jump(tiles[0]);
        step(&mut s, &mut c, &[Action::Confirm]);
        assert_eq!(s.mode(), &Mode::default());
        let archer = s.state().unit(UnitId(3)).unwrap();
        assert_eq!((archer.pos, archer.acted), (tiles[0], true));
    }
}
