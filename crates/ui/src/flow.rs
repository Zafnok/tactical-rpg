//! The game flow (ticket 0801): from `New Game` through each chapter's
//! scenes and battle to the result; and saving and loading it (0802).
//!
//! ```text
//! New Game → mode → lead → [chapter: intro scenes → battle
//!     ├─ victory → apply the result → victory scenes
//!     │            → "Save your progress?" (→ slot picker)
//!     │            → next chapter, or "To be continued" → title
//!     ├─ defeat  → Game Over → Retry (the battle again) | Title
//!     └─ Suspend → the suspend save → title]
//! Load Game → slot picker → the chapter after the one the save cleared
//! Continue  → the suspended battle, where it was (the save is deleted)
//! ```
//!
//! [`FlowScreen`] is one screen on the stack that owns the campaign and
//! hosts the flow's screens itself, one at a time ([`Stage`]): it updates
//! and draws the current one, passes on what it pushes (the battle's scene
//! overlays), and when it pops reads its result and moves on. Its
//! [`name`](Screen::name) is the current screen's. A battle is played from
//! its [`BattleSetup`], kept so that `Restart Battle` (map menu) and
//! `Retry` (Game Over) rebuild it exactly, every rewind charge back.
//!
//! **Saves** (`death-and-difficulty.md`, ADR-0039). A chapter save holds
//! the campaign once its chapter is won, so its
//! [`chapter`](Campaign::chapter) is the one just cleared until the next
//! begins; loading one starts the chapter after it. The suspend save holds
//! the campaign as the battle started with it and the battle's history;
//! continuing rebuilds the battle's setup from the two, so a restart after
//! it is the same as before.
//!
//! No Preparations yet (0408): every battle uses its default pack (the
//! content validator refuses `preparations: true`).
//!
//! Music (ticket 0807, `docs/design/audio.md`): each battle file names its
//! music, asked for once each time the battle starts (also on `Retry` and
//! `Restart Battle`, where a pool picks again). The battle screen asks for
//! none, so the track stays through both phases, combat and rewinds, and
//! on into the victory scenes. Game Over and "To be continued" stop it.

use std::any::Any;
use std::collections::VecDeque;

use trpg_content::{ChapterDef, Scene, battle_campaign, new_campaign};
use trpg_core::{
    BattleDef, BattleMusic, BattleRewards, BattleSetup, BattleState, Campaign, GameMode, Outcome,
    SaveFile, SavePoint,
};

use crate::glyph_buffer::GlyphBuffer;
use crate::save::{self, SUSPEND_KEY, SaveError};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::screens::game_over::{GameOverChoice, GameOverScreen, ToBeContinuedScreen};
use crate::screens::lead_select::LeadSelectScreen;
use crate::screens::mode_select::ModeSelectScreen;
use crate::screens::save::{SavePromptScreen, SlotOutcome, SlotPickerScreen};
use crate::screens::{BattleScreen, DialogueScreen};

/// Asks for battle `def`'s music: its cue, or a track picked from its
/// pool. The pick is the UI's, not the battle's RNG (ADR-0019): a rewind or
/// a replay never changes the track. An empty pool leaves the music as it
/// is.
fn play_battle_music(ctx: &mut Ctx, def: &BattleDef) {
    let cue = match &def.music {
        BattleMusic::Cue(cue) => Some(cue.clone()),
        BattleMusic::Pool(pool) => ctx.pick_music(pool),
    };
    if let Some(cue) = cue {
        ctx.audio.play_music(&cue);
    }
}

/// The chapter the debug Quick Battle plays (`assets/chapters/quick.ron`).
pub const QUICK_CHAPTER: &str = "quick";

/// The screen the flow shows now.
#[derive(Debug, Clone)]
pub enum Stage {
    /// Classic or Casual.
    Mode(ModeSelectScreen),
    /// The lead's gender and name, in the mode picked.
    Lead(GameMode, LeadSelectScreen),
    /// A chapter's scene.
    Scene(Box<DialogueScreen>),
    /// The chapter's battle.
    Battle(Box<BattleScreen>),
    /// After a defeat.
    GameOver(GameOverScreen),
    /// "Save your progress?", after a chapter's victory scenes.
    SavePrompt(SavePromptScreen),
    /// The slot picker: saving after a victory, or loading from the title.
    Slots(Box<SlotPickerScreen>),
    /// After the last chapter.
    ToBeContinued(ToBeContinuedScreen),
}

/// What comes after the scenes being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    /// The chapter's battle (after the intro scenes).
    Battle,
    /// The next chapter (after the victory scenes).
    NextChapter,
}

/// The chapter's battle, as it started.
#[derive(Debug, Clone)]
struct Fight {
    def: BattleDef,
    setup: BattleSetup,
}

/// The game flow: one chapter after another. See the module docs.
#[derive(Debug, Clone)]
pub struct FlowScreen {
    stage: Stage,
    campaign: Option<Campaign>,
    chapter: Option<ChapterDef>,
    fight: Option<Fight>,
    /// Scenes still to play before [`then`](Self::then).
    scenes: VecDeque<String>,
    then: Then,
    /// The last won battle's rewards (0810 shows them).
    rewards: Option<BattleRewards>,
    /// [`Ctx::clock_s`] when the campaign began, for its playtime.
    started_at: f64,
}

impl FlowScreen {
    /// `New Game`: the mode screen first.
    pub fn new_game() -> Self {
        Self {
            stage: Stage::Mode(ModeSelectScreen::new()),
            campaign: None,
            chapter: None,
            fight: None,
            scenes: VecDeque::new(),
            then: Then::Battle,
            rewards: None,
            started_at: 0.0,
        }
    }

    /// `Load Game`: the slot picker first. Going back from it ends the
    /// flow.
    pub fn load_game(ctx: &Ctx) -> Self {
        let mut flow = Self::new_game();
        flow.stage = Stage::Slots(Box::new(SlotPickerScreen::load(ctx)));
        flow
    }

    /// `Continue`: the suspended battle, exactly where it was. The suspend
    /// save is deleted (it can't be loaded twice,
    /// `death-and-difficulty.md`); a save that can't be continued is left
    /// alone.
    pub fn resume(ctx: &mut Ctx) -> Result<Self, SaveError> {
        let file = save::read(ctx.storage.as_ref(), SUSPEND_KEY)?.ok_or(SaveError::Missing)?;
        let SaveFile {
            campaign, point, ..
        } = file;
        let SavePoint::Battle(mut history) = point else {
            return Err(SaveError::Corrupt);
        };
        // A chapter or battle the game no longer has can't be continued.
        let chapter = ctx.content.chapters.get(&campaign.chapter);
        let chapter = chapter.cloned().ok_or(SaveError::Corrupt)?;
        let def = ctx.content.battles.get(&chapter.battle);
        let def = def.cloned().ok_or(SaveError::Corrupt)?;
        let tables = ctx.content.tables();
        let setup = campaign.battle_setup(&def, &tables);
        history.restore_tables(
            tables.terrain,
            tables.classes,
            tables.items,
            tables.spells,
            tables.skills,
            tables.arts,
        );
        // If deleting fails the battle still continues.
        let _ = ctx.storage.delete(SUSPEND_KEY);
        // The battle's music again (a pool picks afresh).
        play_battle_music(ctx, &def);
        let mut flow = Self::new_game();
        flow.adopt(ctx, campaign);
        flow.chapter = Some(chapter);
        flow.fight = Some(Fight { def, setup });
        flow.stage = Stage::Battle(Box::new(BattleScreen::resume(*history)));
        Ok(flow)
    }

    /// The debug Quick Battle: the [`QUICK_CHAPTER`] with its battle's own
    /// characters, in Classic. `None` if the content lacks it.
    pub fn quick_battle(ctx: &mut Ctx) -> Option<Self> {
        let chapter = ctx.content.chapters.get(QUICK_CHAPTER)?;
        let def = ctx.content.battles.get(&chapter.battle)?;
        let mut campaign = battle_campaign(&ctx.content, def, GameMode::Classic, ctx.lead.clone());
        QUICK_CHAPTER.clone_into(&mut campaign.chapter);
        let mut flow = Self::new_game();
        flow.begin(ctx, campaign);
        Some(flow)
    }

    /// The flow's current screen.
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// The campaign, once the lead is made.
    pub fn campaign(&self) -> Option<&Campaign> {
        self.campaign.as_ref()
    }

    /// The chapter being played.
    pub fn chapter(&self) -> Option<&ChapterDef> {
        self.chapter.as_ref()
    }

    /// The battle screen, while the battle is on.
    pub fn battle(&self) -> Option<&BattleScreen> {
        match &self.stage {
            Stage::Battle(b) => Some(b),
            _ => None,
        }
    }

    /// The battle screen, while the battle is on, for scripted tests.
    pub fn battle_mut(&mut self) -> Option<&mut BattleScreen> {
        match &mut self.stage {
            Stage::Battle(b) => Some(b),
            _ => None,
        }
    }

    /// What the last won battle gave.
    pub fn rewards(&self) -> Option<&BattleRewards> {
        self.rewards.as_ref()
    }

    /// The current screen, as a [`Screen`].
    fn screen(&self) -> &dyn Screen {
        match &self.stage {
            Stage::Mode(s) => s,
            Stage::Lead(_, s) => s,
            Stage::Scene(s) => s.as_ref(),
            Stage::Battle(s) => s.as_ref(),
            Stage::GameOver(s) => s,
            Stage::SavePrompt(s) => s,
            Stage::Slots(s) => s.as_ref(),
            Stage::ToBeContinued(s) => s,
        }
    }

    fn screen_mut(&mut self) -> &mut dyn Screen {
        match &mut self.stage {
            Stage::Mode(s) => s,
            Stage::Lead(_, s) => s,
            Stage::Scene(s) => s.as_mut(),
            Stage::Battle(s) => s.as_mut(),
            Stage::GameOver(s) => s,
            Stage::SavePrompt(s) => s,
            Stage::Slots(s) => s.as_mut(),
            Stage::ToBeContinued(s) => s,
        }
    }

    /// Makes `campaign` the flow's: dialogue from here on uses its lead,
    /// and its playtime counts on from what it had.
    fn adopt(&mut self, ctx: &mut Ctx, campaign: Campaign) {
        ctx.lead = campaign.lead.clone();
        #[expect(clippy::cast_precision_loss, reason = "exact below 2^53 s")]
        let played = campaign.playtime_s as f64;
        self.started_at = ctx.clock_s - played;
        self.campaign = Some(campaign);
    }

    /// Starts `campaign` at its chapter.
    fn begin(&mut self, ctx: &mut Ctx, campaign: Campaign) {
        let chapter = campaign.chapter.clone();
        self.adopt(ctx, campaign);
        self.start_chapter(ctx, &chapter);
    }

    /// Goes on with the loaded chapter save `campaign`: the chapter after
    /// the one it cleared ("To be continued" if the game has none yet).
    fn begin_loaded(&mut self, ctx: &mut Ctx, campaign: Campaign) {
        self.chapter = ctx.content.chapters.get(&campaign.chapter).cloned();
        self.adopt(ctx, campaign);
        self.next_chapter(ctx);
    }

    /// Starts chapter `id` with its intro scenes. A chapter missing from
    /// the content (validation rules it out) ends the flow at "To be
    /// continued".
    fn start_chapter(&mut self, ctx: &mut Ctx, id: &str) {
        let Some(chapter) = ctx.content.chapters.get(id).cloned() else {
            self.the_end(ctx);
            return;
        };
        self.scenes = chapter.intro_scenes.iter().cloned().collect();
        self.then = Then::Battle;
        self.chapter = Some(chapter);
        self.next_scene(ctx);
    }

    /// Plays the next scene waiting, or goes on to what follows them.
    /// Scenes missing from the content (validation rules it out) are
    /// skipped.
    fn next_scene(&mut self, ctx: &mut Ctx) {
        while let Some(id) = self.scenes.pop_front() {
            if let Some(scene) = ctx.content.dialogue.get(&id) {
                self.play(scene.clone(), ctx);
                return;
            }
        }
        match self.then {
            Then::Battle => self.start_battle(ctx),
            // The chapter is over: the save prompt, then the next one.
            Then::NextChapter => self.stage = Stage::SavePrompt(SavePromptScreen::new()),
        }
    }

    fn play(&mut self, scene: Scene, ctx: &Ctx) {
        let lead = self
            .campaign
            .as_ref()
            .map_or_else(|| ctx.lead.clone(), |c| c.lead.clone());
        let names = ctx.content.names.clone();
        self.stage = Stage::Scene(Box::new(DialogueScreen::new(scene, lead, names)));
    }

    /// Starts the chapter's battle with the campaign's army.
    fn start_battle(&mut self, ctx: &mut Ctx) {
        let def = self
            .chapter
            .as_ref()
            .and_then(|c| ctx.content.battles.get(&c.battle));
        let (Some(def), Some(campaign)) = (def, &self.campaign) else {
            self.the_end(ctx);
            return;
        };
        let setup = campaign.battle_setup(def, &ctx.content.tables());
        self.fight = Some(Fight {
            def: def.clone(),
            setup,
        });
        self.restart(ctx);
    }

    /// (Re)starts the battle from its setup: turn 1, every rewind charge,
    /// and its music (a pool picks again).
    fn restart(&mut self, ctx: &mut Ctx) {
        let Some(fight) = &self.fight else {
            return;
        };
        play_battle_music(ctx, &fight.def);
        let (state, events) = BattleState::new(fight.setup.clone());
        self.stage = Stage::Battle(Box::new(BattleScreen::start(state, &events)));
    }

    /// "To be continued", in silence.
    fn the_end(&mut self, ctx: &mut Ctx) {
        ctx.audio.stop_music();
        self.stage = Stage::ToBeContinued(ToBeContinuedScreen);
    }

    /// The battle screen closed: a restart, a suspend, or its outcome.
    /// Game Over is silent (0809 gives it its own music). Returns `true`
    /// when the flow is over (suspended: back to the title).
    fn battle_over(&mut self, ctx: &mut Ctx, battle: Box<BattleScreen>) -> bool {
        if battle.restart_requested() {
            self.restart(ctx);
            return false;
        }
        if battle.suspend_requested() {
            return self.suspend(ctx, battle);
        }
        if battle.state().outcome() == Some(Outcome::Victory) {
            self.won(ctx, &battle);
        } else {
            ctx.audio.stop_music();
            self.stage = Stage::GameOver(GameOverScreen::new());
        }
        false
    }

    /// Writes the suspend save: the campaign as the battle started with it
    /// and the battle's history. Returns whether it was written; if not,
    /// the battle goes on, showing why.
    fn suspend(&mut self, ctx: &mut Ctx, mut battle: Box<BattleScreen>) -> bool {
        let written = match &self.campaign {
            Some(campaign) => {
                let file = SaveFile::suspended(campaign.clone(), battle.history().clone());
                save::write(ctx.storage.as_mut(), SUSPEND_KEY, &file)
            }
            None => Err(SaveError::Missing),
        };
        match written {
            Ok(()) => true,
            Err(e) => {
                battle.suspend_failed(e.to_string());
                self.stage = Stage::Battle(battle);
                false
            }
        }
    }

    /// Applies the won battle to the campaign, then the victory scenes.
    fn won(&mut self, ctx: &mut Ctx, battle: &BattleScreen) {
        if let (Some(campaign), Some(fight)) = (&mut self.campaign, &self.fight) {
            let charges = battle.history().charges_left();
            self.rewards = campaign
                .apply_result(&fight.def, battle.state(), charges)
                .ok();
        }
        self.scenes = self
            .chapter
            .iter()
            .flat_map(|c| c.victory_scenes.iter().cloned())
            .collect();
        self.then = Then::NextChapter;
        self.next_scene(ctx);
    }

    /// After a chapter's save prompt (or loading its save): its next
    /// chapter, or "To be continued".
    fn next_chapter(&mut self, ctx: &mut Ctx) {
        let next = self.chapter.as_ref().and_then(|c| c.next.clone());
        match next {
            Some(id) => {
                if let Some(campaign) = &mut self.campaign {
                    campaign.chapter.clone_from(&id);
                }
                self.start_chapter(ctx, &id);
            }
            None => self.the_end(ctx),
        }
    }

    /// The current screen popped: move on. Returns `true` when the flow is
    /// over (back to the title).
    fn advance(&mut self, ctx: &mut Ctx) -> bool {
        let stage = std::mem::replace(&mut self.stage, Stage::ToBeContinued(ToBeContinuedScreen));
        match stage {
            Stage::Mode(s) => match s.result() {
                Some(mode) => self.stage = Stage::Lead(mode, LeadSelectScreen::new()),
                None => return true,
            },
            Stage::Lead(mode, s) => match s.result() {
                Some(lead) => {
                    let campaign = new_campaign(&ctx.content, mode, lead.clone());
                    self.begin(ctx, campaign);
                }
                None => self.stage = Stage::Mode(ModeSelectScreen::new()),
            },
            Stage::Scene(_) => self.next_scene(ctx),
            Stage::Battle(b) => return self.battle_over(ctx, b),
            Stage::GameOver(s) => match s.result() {
                Some(GameOverChoice::Retry) => self.restart(ctx),
                _ => return true,
            },
            Stage::SavePrompt(s) => match (s.result(), &self.campaign) {
                (Some(true), Some(campaign)) => {
                    let picker = SlotPickerScreen::save(ctx, campaign.clone());
                    self.stage = Stage::Slots(Box::new(picker));
                }
                _ => self.next_chapter(ctx),
            },
            Stage::Slots(s) => {
                let saving = s.is_saving();
                match s.into_result() {
                    Some(SlotOutcome::Saved(_)) => self.next_chapter(ctx),
                    Some(SlotOutcome::Loaded(_, file)) => self.begin_loaded(ctx, file.campaign),
                    // Back from saving: the question again. Back from
                    // loading: the title.
                    None if saving => self.stage = Stage::SavePrompt(SavePromptScreen::new()),
                    None => return true,
                }
            }
            Stage::ToBeContinued(_) => return true,
        }
        false
    }

    /// Brings the campaign's playtime up to date.
    fn count_playtime(&mut self, ctx: &Ctx) {
        if let Some(campaign) = &mut self.campaign {
            let played = (ctx.clock_s - self.started_at).max(0.0);
            // Whole seconds; a campaign never runs anywhere near 2^53 s.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "non-negative, far below u64::MAX"
            )]
            let played = played as u64;
            campaign.playtime_s = played;
        }
    }
}

/// Every screen the flow hosts is opaque (a battle's scene overlays are
/// pushed on the stack, not hosted), so the flow is too.
impl Screen for FlowScreen {
    fn name(&self) -> &'static str {
        self.screen().name()
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        self.count_playtime(ctx);
        match self.screen_mut().update(ctx, input) {
            Transition::Pop => {
                if self.advance(ctx) {
                    return Transition::Pop;
                }
                Transition::None
            }
            // The flow's screens only push overlays (a battle's scenes).
            other => other,
        }
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        self.screen().draw(ctx, buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
