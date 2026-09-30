//! Playing a dialogue [`Scene`] one text box at a time. [`DialoguePlayer`]
//! is the state machine only (who stands where, what is said, which reply
//! choice is open); the dialogue screen (ticket 0704) draws its [`View`].

use std::borrow::Cow;

use trpg_content::{ChoiceOption, Scene, Side, Step};
use trpg_core::{CharacterId, LeadProfile};

/// The expression a character who came on during a reply's reaction shows
/// once the scene rejoins (every portrait has it).
const REJOIN_EXPRESSION: &str = "neutral";

/// A character standing on one side of the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placed {
    character: CharacterId,
    expression: String,
}

impl Placed {
    fn portrait(&self) -> Portrait<'_> {
        Portrait {
            character: &self.character,
            expression: &self.expression,
        }
    }
}

/// A portrait on screen: who, with which expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Portrait<'a> {
    /// The character.
    pub character: &'a CharacterId,
    /// Their current expression.
    pub expression: &'a str,
}

/// What the screen shows for the current text box, with the lead's name
/// and pronouns filled in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct View<'a> {
    /// The left portrait, if any.
    pub left: Option<Portrait<'a>>,
    /// The right portrait, if any.
    pub right: Option<Portrait<'a>>,
    /// Which side is speaking (`None` for narration, or when finished).
    pub speaker: Option<Side>,
    /// The text box's text (`None` once the scene is finished). While a
    /// choice is open, the text box before it (the line being answered).
    pub text: Option<Cow<'a, str>>,
    /// The location/time caption, if one has been shown.
    pub caption: Option<Cow<'a, str>>,
    /// Whether the text is narration (no speaker).
    pub narration: bool,
    /// The lead's replies, in menu order, while a choice is open: pick
    /// one with [`DialoguePlayer::choose`].
    pub choices: Option<Vec<Cow<'a, str>>>,
}

/// Where a step is: in the scene, or in a reply's reaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct At {
    /// Index in the scene's steps (for a reaction: of its `Choice`).
    step: usize,
    /// For a reaction: the option and the step's index in it.
    reaction: Option<(usize, usize)>,
}

/// What is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Showing {
    /// A text box: the speech or narration step at `At`.
    Text(At),
    /// The choice at this scene step, answering the text box `At` (if any).
    Choice(usize, Option<At>),
    /// The scene is over.
    Finished,
}

/// A reply being played: its reaction, and the expressions to restore when
/// the scene rejoins.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Reaction {
    /// The scene step of the `Choice`.
    choice: usize,
    /// The option picked.
    option: usize,
    /// The next reaction step.
    next: usize,
    /// Who stood on screen when the choice opened.
    before: [Option<Placed>; 2],
}

/// Plays a scene: each [`advance`](Self::advance) moves to the next text
/// box (speech or narration), applying the portrait and caption steps in
/// between. At a reply choice it waits for [`choose`](Self::choose), plays
/// that reply's reaction, then rejoins the scene after the choice. Pure: no
/// drawing, no clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialoguePlayer {
    scene: Scene,
    /// Fills in the lead's name and pronouns.
    lead: LeadProfile,
    /// Index of the next scene step (after the text box on screen, or the
    /// open choice's).
    next: usize,
    /// The reply being played, if any.
    reaction: Option<Reaction>,
    showing: Showing,
    /// The last text box shown, for an open choice.
    last_text: Option<At>,
    left: Option<Placed>,
    right: Option<Placed>,
    caption: Option<String>,
}

impl DialoguePlayer {
    /// Starts `scene`, at its first text box, with `lead`'s name and
    /// pronouns in its text.
    pub fn new(scene: Scene, lead: LeadProfile) -> Self {
        let mut player = Self {
            scene,
            lead,
            next: 0,
            reaction: None,
            showing: Showing::Finished,
            last_text: None,
            left: None,
            right: None,
            caption: None,
        };
        player.step_on();
        player
    }

    /// The lead whose name and pronouns fill the text.
    pub fn lead(&self) -> &LeadProfile {
        &self.lead
    }

    /// The id of the scene being played.
    pub fn scene_id(&self) -> &str {
        &self.scene.id
    }

    /// The step at `at`.
    fn step(&self, at: At) -> Option<&Step> {
        let step = self.scene.steps.get(at.step)?;
        match (step, at.reaction) {
            (_, None) => Some(step),
            (Step::Choice { options }, Some((option, i))) => options.get(option)?.steps.get(i),
            _ => None,
        }
    }

    /// The options of the choice at scene step `index`.
    fn options(&self, index: usize) -> &[ChoiceOption] {
        match self.scene.steps.get(index) {
            Some(Step::Choice { options }) => options,
            _ => &[],
        }
    }

    /// What to show now.
    pub fn current(&self) -> View<'_> {
        let (text_at, choices) = match self.showing {
            Showing::Text(at) => (Some(at), None),
            Showing::Choice(index, before) => {
                let options = self.options(index);
                let texts = options.iter().map(|o| self.lead.substitute(&o.text));
                (before, Some(texts.collect()))
            }
            Showing::Finished => (None, None),
        };
        let step = text_at.and_then(|at| self.step(at));
        let speaker = match step {
            Some(Step::Say { speaker, .. }) => self.side_of(speaker),
            _ => None,
        };
        View {
            left: self.left.as_ref().map(Placed::portrait),
            right: self.right.as_ref().map(Placed::portrait),
            speaker,
            text: step.and_then(Step::text).map(|t| self.lead.substitute(t)),
            caption: self.caption.as_deref().map(|c| self.lead.substitute(c)),
            narration: matches!(step, Some(Step::Narrate { .. })),
            choices,
        }
    }

    /// Moves to the next text box, applying every portrait and caption step
    /// before it. After the last one the scene is finished; advancing a
    /// finished scene does nothing. While a choice is open this does
    /// nothing either: [`choose`](Self::choose) a reply.
    pub fn advance(&mut self) {
        if !matches!(self.showing, Showing::Choice(..)) {
            self.step_on();
        }
    }

    /// Picks reply `option` of the open choice: plays its reaction, then
    /// the scene after the choice. Does nothing if no choice is open or
    /// there is no such option.
    pub fn choose(&mut self, option: usize) {
        let Showing::Choice(index, _) = self.showing else {
            return;
        };
        if option >= self.options(index).len() {
            return;
        }
        self.reaction = Some(Reaction {
            choice: index,
            option,
            next: 0,
            before: [self.left.clone(), self.right.clone()],
        });
        self.step_on();
    }

    /// Advances until a choice is open or the scene is over (skipping a
    /// scene stops at each choice).
    pub fn skip_to_choice(&mut self) {
        while matches!(self.showing, Showing::Text(_)) {
            self.step_on();
        }
    }

    /// Whether a reply choice is waiting for [`choose`](Self::choose).
    pub fn is_choosing(&self) -> bool {
        matches!(self.showing, Showing::Choice(..))
    }

    /// Whether every text box has been shown and advanced past.
    pub fn is_finished(&self) -> bool {
        self.showing == Showing::Finished
    }

    /// Applies steps up to the next text box or choice.
    fn step_on(&mut self) {
        loop {
            if let Some(r) = self.reaction.as_mut() {
                let at = At {
                    step: r.choice,
                    reaction: Some((r.option, r.next)),
                };
                r.next += 1;
                match self.step(at).cloned() {
                    Some(step) => {
                        if self.apply(&step, at) {
                            return;
                        }
                    }
                    None => self.rejoin(),
                }
                continue;
            }
            let index = self.next;
            let Some(step) = self.scene.steps.get(index).cloned() else {
                self.showing = Showing::Finished;
                return;
            };
            self.next = index + 1;
            if let Step::Choice { options } = &step {
                // A choice without replies (which the validator rejects)
                // is passed over rather than waiting forever.
                if !options.is_empty() {
                    self.showing = Showing::Choice(index, self.last_text);
                    return;
                }
                continue;
            }
            let at = At {
                step: index,
                reaction: None,
            };
            if self.apply(&step, at) {
                return;
            }
        }
    }

    /// Ends the reaction being played: each portrait takes back the
    /// expression it had before the choice ([`REJOIN_EXPRESSION`] for a
    /// character who came on during it), so every reply leaves the same
    /// screen.
    fn rejoin(&mut self) {
        let Some(r) = self.reaction.take() else {
            return;
        };
        for p in [&mut self.left, &mut self.right].into_iter().flatten() {
            let before = r
                .before
                .iter()
                .flatten()
                .find(|b| b.character == p.character);
            p.expression =
                before.map_or_else(|| REJOIN_EXPRESSION.to_owned(), |b| b.expression.clone());
        }
    }

    /// Applies `step` (found at `at`). Returns whether it is a text box,
    /// which is now shown.
    fn apply(&mut self, step: &Step, at: At) -> bool {
        match step {
            Step::Caption { text } => self.caption = Some(text.clone()),
            Step::Place {
                side,
                character,
                expression,
            } => {
                let placed = Some(Placed {
                    character: character.clone(),
                    expression: expression.clone(),
                });
                match side {
                    Side::Left => self.left = placed,
                    Side::Right => self.right = placed,
                }
            }
            Step::Clear { side: Side::Left } => self.left = None,
            Step::Clear { side: Side::Right } => self.right = None,
            Step::Say {
                speaker,
                expression,
                ..
            } => {
                if let Some(expression) = expression {
                    for p in [&mut self.left, &mut self.right].into_iter().flatten() {
                        if &p.character == speaker {
                            p.expression.clone_from(expression);
                        }
                    }
                }
            }
            Step::Narrate { .. } | Step::Choice { .. } => {}
        }
        if step.text().is_some() {
            self.showing = Showing::Text(at);
            self.last_text = Some(at);
            return true;
        }
        false
    }

    /// The side `character` stands on.
    fn side_of(&self, character: &CharacterId) -> Option<Side> {
        let on = |p: &Option<Placed>| p.as_ref().is_some_and(|p| &p.character == character);
        if on(&self.left) {
            Some(Side::Left)
        } else if on(&self.right) {
            Some(Side::Right)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
