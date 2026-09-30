//! Enemy and Other phase playback (ticket 0502): the battle screen asks
//! [`trpg_core::next_command`] for each AI action, applies it, and shows it
//! as an [`AiAction`]: the camera pans to the unit, the cursor marks it for
//! a moment, it walks its path, then its combat plays
//! ([`Mode::Combat`]) as the player's do. The
//! battle already holds the result while the action is shown; until the
//! walk ends, the map shows the units as they were before it.
//!
//! Everything here is a pure function of the action and its clock
//! (ADR-0004 rule 3); the screen owns the camera and cursor and copies them
//! from it each frame.

use trpg_core::{Pos, Unit, UnitId};

use super::mode::{Mode, WALK_TILES_PER_S};

/// How long each part of an AI action takes. *Tunable.*
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pacing {
    /// The camera sliding over to the acting unit, in seconds (none if it
    /// is already in view).
    pub pan: f32,
    /// The cursor on the unit before it moves, in seconds.
    pub highlight: f32,
    /// Walking speed, in tiles per second.
    pub walk_tiles_per_s: f32,
    /// Speed-up while Confirm is held (the combat playback's is
    /// [`Timings::fast`](super::playback::Timings::fast)).
    pub fast: f32,
}

/// The game's pacing (ticket 0502: ~0.25 s pan, 0.2 s highlight; walks as
/// fast as the player's; ×4 while Confirm is held).
pub const PACING: Pacing = Pacing {
    pan: 0.25,
    highlight: 0.2,
    walk_tiles_per_s: WALK_TILES_PER_S,
    fast: 4.0,
};

/// One AI action being shown, before its combat (if any) plays.
#[derive(Debug, Clone, PartialEq)]
pub struct AiAction {
    /// The unit acting.
    unit: UnitId,
    /// The units before the action: what the map shows until the walk
    /// ends.
    before: Vec<Unit>,
    /// The camera's origin when the action starts, and once panned.
    pan: (Pos, Pos),
    /// The unit's move, its start tile first (just that tile if it stays).
    path: Vec<Pos>,
    /// Seconds played.
    t: f32,
    /// What follows: the combat's playback, or browsing.
    then: Mode,
    pacing: Pacing,
}

impl AiAction {
    /// `unit`'s action, from the units `before` it: the camera pans from
    /// origin `from` to `to`, then `unit` walks `path` (its start first;
    /// empty means it stays where it is), then `then`.
    pub fn new(
        unit: UnitId,
        before: Vec<Unit>,
        (from, to): (Pos, Pos),
        path: Vec<Pos>,
        then: Mode,
        pacing: Pacing,
    ) -> Self {
        let path = if path.is_empty() {
            before
                .iter()
                .find(|u| u.id == unit)
                .map(|u| vec![u.pos])
                .unwrap_or_default()
        } else {
            path
        };
        Self {
            unit,
            before,
            pan: (from, to),
            path,
            t: 0.0,
            then,
            pacing,
        }
    }

    /// The unit acting.
    pub fn unit(&self) -> UnitId {
        self.unit
    }

    /// Seconds played.
    pub fn time(&self) -> f32 {
        self.t
    }

    /// How long the pan lasts: none if the camera doesn't move.
    fn pan_len(&self) -> f32 {
        if self.pan.0 == self.pan.1 {
            0.0
        } else {
            self.pacing.pan
        }
    }

    /// When the walk starts.
    pub fn walk_start(&self) -> f32 {
        self.pan_len() + self.pacing.highlight
    }

    /// How long the whole action takes.
    pub fn total(&self) -> f32 {
        let steps = self.path.len().saturating_sub(1);
        let steps = u16::try_from(steps).unwrap_or(u16::MAX);
        self.walk_start() + f32::from(steps) / self.pacing.walk_tiles_per_s
    }

    /// Whether it has played out.
    pub fn done(&self) -> bool {
        self.t >= self.total()
    }

    /// The time [`tick`](Self::tick) would advance the clock to.
    fn advanced(&self, dt: f32, confirm_held: bool) -> f32 {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let speed = if confirm_held { self.pacing.fast } else { 1.0 };
        (self.t + dt * speed).min(self.total())
    }

    /// Advances the clock by `dt` seconds (`confirm_held`: Confirm is down
    /// this frame): [`Pacing::fast`] times faster while held. A bad `dt`
    /// counts as 0.
    pub fn tick(&mut self, dt: f32, confirm_held: bool) {
        self.t = self.advanced(dt, confirm_held);
    }

    /// How many path steps the unit has taken at time `t`.
    fn steps_at(&self, t: f32) -> usize {
        let walked = (t - self.walk_start()) * self.pacing.walk_tiles_per_s;
        let steps = self.path.len().saturating_sub(1);
        (1..=u16::try_from(steps).unwrap_or(u16::MAX))
            .take_while(|&k| f32::from(k) <= walked)
            .count()
    }

    /// How many tiles the unit enters in the next [`tick`](Self::tick)
    /// with the same arguments (for its step sounds).
    pub fn tiles_entered(&self, dt: f32, confirm_held: bool) -> usize {
        self.steps_at(self.advanced(dt, confirm_held)) - self.steps_at(self.t)
    }

    /// Where the unit is drawn: along its path.
    pub fn walker_pos(&self) -> Pos {
        let at = self.path.get(self.steps_at(self.t)).copied();
        at.or_else(|| self.path.first().copied())
            .unwrap_or_default()
    }

    /// The tile the action starts on (the cursor marks it).
    pub fn start(&self) -> Pos {
        self.path.first().copied().unwrap_or_default()
    }

    /// Whether the unit is walking (the camera follows it).
    pub fn walking(&self) -> bool {
        self.t >= self.walk_start()
    }

    /// Whether the cursor marks the unit: after the pan, until it walks.
    pub fn shows_cursor(&self) -> bool {
        self.t >= self.pan_len() && !self.walking()
    }

    /// The camera's origin during the pan (it slides a tile at a time, in
    /// a straight line); where it ends up afterwards.
    pub fn camera(&self) -> Pos {
        let len = self.pan_len();
        if len <= 0.0 || self.t >= len {
            return self.pan.1;
        }
        let k = self.t / len;
        let (from, to) = self.pan;
        let lerp = |a: i32, b: i32| {
            // Tile offsets are small: exact in f32.
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let d = ((b - a) as f32 * k).round() as i32;
            a + d
        };
        Pos::new(lerp(from.x, to.x), lerp(from.y, to.y))
    }

    /// The units as drawn: as they were before the action, the acting unit
    /// on its way along the path.
    pub fn units(&self) -> Vec<Unit> {
        let pos = self.walker_pos();
        self.before
            .iter()
            .map(|u| {
                let mut u = u.clone();
                if u.id == self.unit {
                    u.pos = pos;
                }
                u
            })
            .collect()
    }

    /// What follows the action once shown.
    pub fn into_then(self) -> Mode {
        self.then
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::testing::quick_units;

    fn unit(id: u32, pos: Pos) -> Unit {
        let (_, units) = quick_units(&ctx());
        let mut u = units[0].clone();
        u.id = UnitId(id);
        u.pos = pos;
        u
    }

    fn action(pan: (Pos, Pos), path: Vec<Pos>) -> AiAction {
        let before = vec![unit(4, Pos::new(1, 1)), unit(5, Pos::new(3, 3))];
        AiAction::new(UnitId(4), before, pan, path, Mode::default(), PACING)
    }

    fn line(n: i32) -> Vec<Pos> {
        (0..=n).map(|x| Pos::new(1 + x, 1)).collect()
    }

    #[test]
    fn pans_then_highlights_then_walks() {
        let still = Pos::new(1, 2);
        let mut a = action((still, Pos::new(5, 0)), line(3));
        let walk = 3.0 / PACING.walk_tiles_per_s;
        assert!((a.walk_start() - (PACING.pan + PACING.highlight)).abs() < 1e-6);
        assert!((a.total() - (a.walk_start() + walk)).abs() < 1e-6);
        // The pan: a tile at a time, in a straight line.
        assert_eq!(a.camera(), still);
        assert!(!a.shows_cursor());
        a.tick(PACING.pan / 2.0, false);
        assert_eq!(a.camera(), Pos::new(3, 1));
        assert!(!a.shows_cursor());
        // The highlight: the cursor on the unit, still on its tile.
        a.tick(PACING.pan / 2.0 + 0.01, false);
        assert_eq!(a.camera(), Pos::new(5, 0));
        assert!(a.shows_cursor());
        assert!(!a.walking());
        assert_eq!(a.walker_pos(), Pos::new(1, 1));
        // The walk.
        a.tick(PACING.highlight, false);
        assert!(a.walking());
        assert!(!a.shows_cursor());
        assert_eq!(a.tiles_entered(walk, false), 3);
        a.tick(1.5 / PACING.walk_tiles_per_s, false);
        assert_eq!(a.walker_pos(), Pos::new(2, 1));
        // One step taken, two to go.
        assert_eq!(a.tiles_entered(walk, false), 2);
        assert_eq!(a.units()[0].pos, Pos::new(2, 1));
        assert_eq!(a.units()[1].pos, Pos::new(3, 3), "others stay");
        assert!(!a.done());
        a.tick(10.0, false);
        assert!(a.done());
        assert_eq!(a.walker_pos(), Pos::new(4, 1));
        assert!((a.time() - a.total()).abs() < 1e-6);
        // The camera stays where the pan ended.
        assert_eq!(a.camera(), Pos::new(5, 0));
    }

    #[test]
    fn no_pan_when_the_camera_stays_and_no_walk_when_the_unit_stays() {
        let here = Pos::new(2, 2);
        let a = action((here, here), vec![]);
        assert!((a.total() - PACING.highlight).abs() < 1e-6);
        assert!(a.shows_cursor());
        assert_eq!(a.start(), Pos::new(1, 1));
        assert_eq!(a.walker_pos(), Pos::new(1, 1));
        assert_eq!(a.tiles_entered(1.0, false), 0);
    }

    #[test]
    fn holding_confirm_plays_four_times_as_fast() {
        let mut slow = action((Pos::new(0, 0), Pos::new(3, 0)), line(4));
        let mut fast = slow.clone();
        slow.tick(0.2, false);
        fast.tick(0.05, true);
        assert!((slow.time() - fast.time()).abs() < 1e-6);
        // Bad times count as nothing.
        fast.tick(f32::NAN, true);
        fast.tick(-1.0, false);
        assert!((slow.time() - fast.time()).abs() < 1e-6);
    }

    #[test]
    fn ends_in_what_follows() {
        let a = action((Pos::new(0, 0), Pos::new(0, 0)), line(1));
        assert_eq!(a.unit(), UnitId(4));
        assert_eq!(a.into_then(), Mode::default());
    }
}
