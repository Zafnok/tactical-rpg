//! Phase and outcome banners (ticket 0405, `docs/design/turn-structure.md`):
//! `PLAYER PHASE` / `ENEMY PHASE` / `OTHER PHASE` with the turn beneath when
//! a phase starts, `VICTORY` / `DEFEAT` when the battle ends. Centred on the
//! map view in a double box.

use trpg_core::{Event, Outcome, Phase, Turn};

use super::layout::MAP_VIEW;
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};

/// How long a phase banner stays up unless Confirm closes it, in seconds.
/// *Tunable.*
pub const PHASE_BANNER_S: f32 = 1.0;

/// Banner box size, in cells.
pub const BANNER_W: i32 = 24;

/// Banner box height: border, blank, title, second line, blank, border.
pub const BANNER_H: i32 = 6;

/// What a banner announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerKind {
    /// A phase started.
    Phase {
        /// The phase.
        phase: Phase,
        /// The turn.
        turn: Turn,
    },
    /// The battle ended; stays until confirmed.
    Outcome(Outcome),
}

/// A banner on screen, and how long it has been up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Banner {
    /// What it announces.
    pub kind: BannerKind,
    /// Seconds shown so far.
    pub t: f32,
}

impl Banner {
    /// The banner for `event`, if it has one.
    pub fn for_event(event: &Event) -> Option<Self> {
        let kind = match *event {
            Event::PhaseStarted { turn, phase } => BannerKind::Phase { phase, turn },
            Event::BattleEnded { outcome } => BannerKind::Outcome(outcome),
            _ => return None,
        };
        Some(Self { kind, t: 0.0 })
    }

    /// Whether it has been up long enough to close by itself (outcome
    /// banners never do).
    pub fn expired(&self) -> bool {
        matches!(self.kind, BannerKind::Phase { .. }) && self.t >= PHASE_BANNER_S
    }

    /// The big line and its colour.
    pub fn title(&self) -> (&'static str, UiColor) {
        match self.kind {
            BannerKind::Phase { phase, .. } => match phase {
                Phase::Player => ("PLAYER PHASE", UiColor::Player),
                Phase::Enemy => ("ENEMY PHASE", UiColor::Enemy),
                Phase::Other => ("OTHER PHASE", UiColor::Ally),
            },
            BannerKind::Outcome(Outcome::Victory) => ("VICTORY", UiColor::TextHighlight),
            BannerKind::Outcome(Outcome::Defeat) => ("DEFEAT", UiColor::HpLow),
        }
    }

    /// The line beneath: the turn for a phase (`Turn 2`, or `Turn 2/8`
    /// with a limit), nothing for an outcome.
    pub fn subtitle(&self, limit: Option<Turn>) -> String {
        match self.kind {
            BannerKind::Phase { turn, .. } => match limit {
                Some(limit) => format!("Turn {turn}/{limit}"),
                None => format!("Turn {turn}"),
            },
            BannerKind::Outcome(_) => String::new(),
        }
    }

    /// Draws the banner centred on the map view; `limit` is the map's turn
    /// limit, if any.
    pub fn draw(&self, buf: &mut GlyphBuffer, palette: &Palette, limit: Option<Turn>) {
        let c = |u| palette.get(u);
        let bg = c(UiColor::PanelBg);
        let rect = Rect::new(
            MAP_VIEW.x + (MAP_VIEW.w - BANNER_W) / 2,
            MAP_VIEW.y + (MAP_VIEW.h - BANNER_H) / 2,
            BANNER_W,
            BANNER_H,
        );
        let (title, color) = self.title();
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, c(color), bg);
        let centred = |text: &str| {
            let w = i32::try_from(text.chars().count()).unwrap_or(0);
            rect.x + (rect.w - w) / 2
        };
        buf.print(centred(title), rect.y + 2, title, c(color), bg);
        let sub = self.subtitle(limit);
        buf.print(centred(&sub), rect.y + 3, &sub, c(UiColor::Text), bg);
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::UnitId;

    use super::*;

    fn phase(phase: Phase, turn: Turn) -> Banner {
        Banner {
            kind: BannerKind::Phase { phase, turn },
            t: 0.0,
        }
    }

    #[test]
    fn events_with_banners() {
        assert_eq!(
            Banner::for_event(&Event::PhaseStarted {
                turn: 2,
                phase: Phase::Enemy
            }),
            Some(phase(Phase::Enemy, 2))
        );
        let ended = Banner::for_event(&Event::BattleEnded {
            outcome: Outcome::Defeat,
        });
        assert_eq!(
            ended.map(|b| b.kind),
            Some(BannerKind::Outcome(Outcome::Defeat))
        );
        assert_eq!(
            Banner::for_event(&Event::UnitActed { unit: UnitId(1) }),
            None
        );
    }

    #[test]
    fn titles_colours_and_turns() {
        let titles = [Phase::Player, Phase::Enemy, Phase::Other].map(|p| phase(p, 1).title());
        assert_eq!(
            titles,
            [
                ("PLAYER PHASE", UiColor::Player),
                ("ENEMY PHASE", UiColor::Enemy),
                ("OTHER PHASE", UiColor::Ally),
            ]
        );
        assert_eq!(phase(Phase::Player, 3).subtitle(None), "Turn 3");
        assert_eq!(phase(Phase::Player, 3).subtitle(Some(8)), "Turn 3/8");
        let won = Banner {
            kind: BannerKind::Outcome(Outcome::Victory),
            t: 0.0,
        };
        assert_eq!(won.title(), ("VICTORY", UiColor::TextHighlight));
        assert_eq!(won.subtitle(Some(8)), "");
    }

    #[test]
    fn only_phase_banners_expire() {
        let mut b = phase(Phase::Player, 1);
        b.t = PHASE_BANNER_S * 0.99;
        assert!(!b.expired());
        b.t = PHASE_BANNER_S;
        assert!(b.expired());
        let lost = Banner {
            kind: BannerKind::Outcome(Outcome::Defeat),
            t: 100.0,
        };
        assert!(!lost.expired());
        assert_eq!(lost.title(), ("DEFEAT", UiColor::HpLow));
    }
}
