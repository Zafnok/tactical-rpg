//! The game flow (ticket 0801): from `New Game` through each chapter's
//! scenes and battle to the result.
//!
//! ```text
//! New Game → mode → lead → [chapter: intro scenes → (Preparations) → battle
//!     ├─ victory → apply the result → victory scenes → (0802: save prompt)
//!     │            → next chapter, or "To be continued" → title
//!     └─ defeat  → Game Over → Retry (the battle again) | Title]
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
//! A battle with `preparations: true` opens the Preparations screen (0408)
//! first, which changes the setup's loadouts and pack; `Fight!` starts the
//! battle with it. Both restarts go back to Preparations, as the player
//! left it (Nick, 0408), so they can change their gear before trying again.
//! Only the Quick Battle may leave Preparations (back to the title).

use std::any::Any;
use std::collections::VecDeque;

use trpg_content::{ChapterDef, Scene, battle_campaign, new_campaign};
use trpg_core::{BattleDef, BattleRewards, BattleSetup, BattleState, Campaign, GameMode, Outcome};

use crate::glyph_buffer::GlyphBuffer;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::screens::game_over::{GameOverChoice, GameOverScreen, ToBeContinuedScreen};
use crate::screens::lead_select::LeadSelectScreen;
use crate::screens::mode_select::ModeSelectScreen;
use crate::screens::preparations::{PrepOutcome, PreparationsScreen};
use crate::screens::{BattleScreen, DialogueScreen};

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
    /// Loadouts and the pack, before a battle that has Preparations.
    Preparations(Box<PreparationsScreen>),
    /// The chapter's battle.
    Battle(Box<BattleScreen>),
    /// After a defeat.
    GameOver(GameOverScreen),
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

/// The chapter's battle, as it started (as last prepared, if it has
/// Preparations).
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
    /// Whether Preparations may be left (the Quick Battle: to the title).
    can_leave: bool,
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
            can_leave: false,
        }
    }

    /// The debug Quick Battle: the [`QUICK_CHAPTER`] with its battle's own
    /// characters, in Classic. `None` if the content lacks it.
    pub fn quick_battle(ctx: &mut Ctx) -> Option<Self> {
        let chapter = ctx.content.chapters.get(QUICK_CHAPTER)?;
        let def = ctx.content.battles.get(&chapter.battle)?;
        let mut campaign = battle_campaign(&ctx.content, def, GameMode::Classic, ctx.lead.clone());
        QUICK_CHAPTER.clone_into(&mut campaign.chapter);
        let mut flow = Self::new_game();
        flow.can_leave = true;
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

    /// The Preparations screen, while it is open.
    pub fn preparations(&self) -> Option<&PreparationsScreen> {
        match &self.stage {
            Stage::Preparations(p) => Some(p),
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
            Stage::Preparations(s) => s.as_ref(),
            Stage::Battle(s) => s.as_ref(),
            Stage::GameOver(s) => s,
            Stage::ToBeContinued(s) => s,
        }
    }

    fn screen_mut(&mut self) -> &mut dyn Screen {
        match &mut self.stage {
            Stage::Mode(s) => s,
            Stage::Lead(_, s) => s,
            Stage::Scene(s) => s.as_mut(),
            Stage::Preparations(s) => s.as_mut(),
            Stage::Battle(s) => s.as_mut(),
            Stage::GameOver(s) => s,
            Stage::ToBeContinued(s) => s,
        }
    }

    /// Starts `campaign` at its chapter; dialogue from here on uses its
    /// lead.
    fn begin(&mut self, ctx: &mut Ctx, campaign: Campaign) {
        ctx.lead = campaign.lead.clone();
        // A loaded campaign (0802) carries on from its playtime.
        #[expect(clippy::cast_precision_loss, reason = "exact below 2^53 s")]
        let played = campaign.playtime_s as f64;
        self.started_at = ctx.clock_s - played;

        let chapter = campaign.chapter.clone();
        self.campaign = Some(campaign);
        self.start_chapter(ctx, &chapter);
    }

    /// Starts chapter `id` with its intro scenes. A chapter missing from
    /// the content (validation rules it out) ends the flow at "To be
    /// continued".
    fn start_chapter(&mut self, ctx: &Ctx, id: &str) {
        let Some(chapter) = ctx.content.chapters.get(id).cloned() else {
            self.stage = Stage::ToBeContinued(ToBeContinuedScreen);
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
    fn next_scene(&mut self, ctx: &Ctx) {
        while let Some(id) = self.scenes.pop_front() {
            if let Some(scene) = ctx.content.dialogue.get(&id) {
                self.play(scene.clone(), ctx);
                return;
            }
        }
        match self.then {
            Then::Battle => self.start_battle(ctx),
            Then::NextChapter => self.next_chapter(ctx),
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
    fn start_battle(&mut self, ctx: &Ctx) {
        let def = self
            .chapter
            .as_ref()
            .and_then(|c| ctx.content.battles.get(&c.battle));
        let (Some(def), Some(campaign)) = (def, &self.campaign) else {
            self.stage = Stage::ToBeContinued(ToBeContinuedScreen);
            return;
        };
        let setup = campaign.battle_setup(def, &ctx.content.tables());
        self.fight = Some(Fight {
            def: def.clone(),
            setup,
        });
        self.restart();
    }

    /// (Re)starts the battle: Preparations first if it has them (as the
    /// player last left them), else straight to turn 1.
    fn restart(&mut self) {
        let Some(fight) = &self.fight else {
            return;
        };
        if fight.def.preparations {
            let screen = PreparationsScreen::new(fight.setup.clone(), self.can_leave);
            self.stage = Stage::Preparations(Box::new(screen));
        } else {
            self.fight();
        }
    }

    /// Starts the battle from its setup: turn 1, every rewind charge.
    fn fight(&mut self) {
        let Some(fight) = &self.fight else {
            return;
        };
        let (state, events) = BattleState::new(fight.setup.clone());
        self.stage = Stage::Battle(Box::new(BattleScreen::start(state, &events)));
    }

    /// Preparations closed: the battle with what the player set up, or
    /// (`true`) the flow is over because they left.
    fn prepared(&mut self, screen: &PreparationsScreen) -> bool {
        if screen.outcome() != Some(PrepOutcome::Fight) {
            return true;
        }
        if let Some(fight) = &mut self.fight {
            fight.setup = screen.setup().clone();
        }
        self.fight();
        false
    }

    /// The battle screen closed: a restart, or its outcome.
    fn battle_over(&mut self, ctx: &Ctx, battle: &BattleScreen) {
        if battle.restart_requested() {
            self.restart();
            return;
        }
        match battle.state().outcome() {
            Some(Outcome::Victory) => self.won(ctx, battle),
            _ => self.stage = Stage::GameOver(GameOverScreen::new()),
        }
    }

    /// Applies the won battle to the campaign, then the victory scenes.
    fn won(&mut self, ctx: &Ctx, battle: &BattleScreen) {
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

    /// After a chapter's victory scenes: its next chapter, or "To be
    /// continued". The save prompt (0802) goes here.
    fn next_chapter(&mut self, ctx: &Ctx) {
        let next = self.chapter.as_ref().and_then(|c| c.next.clone());
        match next {
            Some(id) => {
                if let Some(campaign) = &mut self.campaign {
                    campaign.chapter.clone_from(&id);
                }
                self.start_chapter(ctx, &id);
            }
            None => self.stage = Stage::ToBeContinued(ToBeContinuedScreen),
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
            Stage::Preparations(p) => return self.prepared(&p),
            Stage::Battle(b) => self.battle_over(ctx, &b),
            Stage::GameOver(s) => match s.result() {
                Some(GameOverChoice::Retry) => self.restart(),
                _ => return true,
            },
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
