//! Playing a dialogue [`Scene`] one text box at a time. [`DialoguePlayer`]
//! is the state machine only (who stands where, what is said); the dialogue
//! screen (ticket 0704) draws its [`View`].

use trpg_content::{Scene, Side, Step};
use trpg_core::CharacterId;

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

/// What the screen shows for the current text box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct View<'a> {
    /// The left portrait, if any.
    pub left: Option<Portrait<'a>>,
    /// The right portrait, if any.
    pub right: Option<Portrait<'a>>,
    /// Which side is speaking (`None` for narration, or when finished).
    pub speaker: Option<Side>,
    /// The text box's text (`None` once the scene is finished).
    pub text: Option<&'a str>,
    /// The location/time caption, if one has been shown.
    pub caption: Option<&'a str>,
    /// Whether the text is narration (no speaker).
    pub narration: bool,
}

/// Plays a scene: each [`advance`](Self::advance) moves to the next text
/// box (speech or narration), applying the portrait and caption steps in
/// between. Pure: no drawing, no clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialoguePlayer {
    scene: Scene,
    /// Index of the next step to apply.
    next: usize,
    /// Index of the text step on screen, `None` when finished.
    current: Option<usize>,
    left: Option<Placed>,
    right: Option<Placed>,
    caption: Option<String>,
}

impl DialoguePlayer {
    /// Starts `scene`, at its first text box.
    pub fn new(scene: Scene) -> Self {
        let mut player = Self {
            scene,
            next: 0,
            current: None,
            left: None,
            right: None,
            caption: None,
        };
        player.advance();
        player
    }

    /// The id of the scene being played.
    pub fn scene_id(&self) -> &str {
        &self.scene.id
    }

    /// What to show now.
    pub fn current(&self) -> View<'_> {
        let step = self.current.and_then(|i| self.scene.steps.get(i));
        let speaker = match step {
            Some(Step::Say { speaker, .. }) => self.side_of(speaker),
            _ => None,
        };
        View {
            left: self.left.as_ref().map(Placed::portrait),
            right: self.right.as_ref().map(Placed::portrait),
            speaker,
            text: step.and_then(Step::text),
            caption: self.caption.as_deref(),
            narration: matches!(step, Some(Step::Narrate { .. })),
        }
    }

    /// Moves to the next text box, applying every portrait and caption step
    /// before it. After the last one the scene is finished; advancing a
    /// finished scene does nothing.
    pub fn advance(&mut self) {
        self.current = None;
        while let Some(step) = self.scene.steps.get(self.next) {
            let index = self.next;
            self.next += 1;
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
                    self.current = Some(index);
                    return;
                }
                Step::Narrate { .. } => {
                    self.current = Some(index);
                    return;
                }
            }
        }
    }

    /// Whether every text box has been shown and advanced past.
    pub fn is_finished(&self) -> bool {
        self.current.is_none()
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
