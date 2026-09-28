//! Combat playback (ticket 0404): after an attack is applied, the battle
//! screen plays its [`Event::CombatResolved`]s strike by strike in a box at
//! the top of the map, then fades out the units that fell
//! ([`Event::UnitFell`]). Everything shown comes from the events; the
//! playback only times them.
//!
//! The timeline is a list of [`Beat`]s laid out when the playback starts:
//! for each combat an intro, then per strike the striker's name flashing,
//! then the result (`HIT -7`, `MISS`, `CRITICAL! -21`) while the target's
//! HP bar drains, with a pause between strikes; then one fall per fallen
//! unit and a short hold. Holding Confirm plays it [`Timings::fast`] times
//! as fast; a tap (a press released within [`Timings::tap`]) skips to the
//! end.

use std::collections::BTreeMap;

use trpg_core::{Event, Faction, Side, StatValue, Strike, Unit, UnitId};

use super::layout::MAP_VIEW;
use super::units::{faction_color, hp_fill};
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};

/// How long each part of the playback takes, in seconds. *Tunable.*
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timings {
    /// The box on screen before a combat's first strike.
    pub intro: f32,
    /// The striker's name flashing before each strike.
    pub flash: f32,
    /// Shortest time a strike's result stays up (longer while HP drains).
    pub result: f32,
    /// How fast an HP bar drains (or fills, for Absorb), in HP per second.
    pub drain_hp_per_s: f32,
    /// Pause between strikes.
    pub gap: f32,
    /// A fallen unit fading into the terrain.
    pub fall: f32,
    /// The box stays up after everything, before the playback ends.
    pub outro: f32,
    /// Speed-up while Confirm is held.
    pub fast: f32,
    /// A Confirm press released sooner than this is a tap, which skips.
    pub tap: f32,
    /// Half-period of the striker's name flashing.
    pub blink: f32,
}

/// The game's timings (ticket 0404: ~30 HP/s drain, 0.25 s between
/// strikes, 0.5 s fade, ×4 while Confirm is held).
pub const TIMINGS: Timings = Timings {
    intro: 0.3,
    flash: 0.3,
    result: 0.5,
    drain_hp_per_s: 30.0,
    gap: 0.25,
    fall: 0.5,
    outro: 0.4,
    fast: 4.0,
    tap: 0.2,
    blink: 0.075,
};

/// One side of a combat, as the box shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fighter {
    /// The unit.
    pub unit: UnitId,
    /// Its name.
    pub name: String,
    /// Its faction (the name's colour).
    pub faction: Faction,
    /// Its HP when the combat starts.
    pub start: StatValue,
    /// Its max HP.
    pub max: StatValue,
}

/// One combat ([`Event::CombatResolved`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bout {
    /// Who attacked.
    pub attacker: Fighter,
    /// Who was attacked.
    pub defender: Fighter,
    /// Every strike, in order.
    pub strikes: Vec<Strike>,
}

/// A part of the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beat {
    /// Combat `bout`'s box before its first strike.
    Intro {
        /// Index into [`Playback::bouts`].
        bout: usize,
    },
    /// The striker's name flashes before strike `strike` of `bout`.
    Flash {
        /// Index into [`Playback::bouts`].
        bout: usize,
        /// Index into the bout's strikes.
        strike: usize,
    },
    /// Strike `strike`'s result is shown while the target's HP drains.
    Result {
        /// Index into [`Playback::bouts`].
        bout: usize,
        /// Index into the bout's strikes.
        strike: usize,
    },
    /// A pause after strike `strike` of `bout` (its result stays up).
    Gap {
        /// Index into [`Playback::bouts`].
        bout: usize,
        /// Index into the bout's strikes.
        strike: usize,
    },
    /// Fallen unit `fall` fades out with its message. Death quotes (0705)
    /// go in as beats just before this one.
    Fall {
        /// Index into [`Playback::falls`].
        fall: usize,
    },
    /// The end: everything stays up a moment.
    Outro,
}

/// A beat placed on the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// What happens.
    pub beat: Beat,
    /// When it starts, in seconds from the playback's start.
    pub start: f32,
    /// How long it lasts.
    pub len: f32,
}

/// A combat playback: the combats and falls of one command's events, a
/// timeline and a clock.
#[derive(Debug, Clone, PartialEq)]
pub struct Playback {
    bouts: Vec<Bout>,
    /// Units that fell, as they were when they fell (for drawing their
    /// fade), in [`Event::UnitFell`] order. The hook for death quotes
    /// (0705).
    falls: Vec<Unit>,
    steps: Vec<Step>,
    timings: Timings,
    t: f32,
    /// Seconds Confirm has been held since it was pressed during the
    /// playback, until released.
    press: Option<f32>,
}

impl Playback {
    /// The playback of `events` (one applied command), or `None` if they
    /// hold no combat. `before` is every unit before the command (for the
    /// combatants' names and starting HP); `fallen` is the battle's fallen
    /// units after it (to draw those that fall here).
    pub fn new(
        events: &[Event],
        before: &[Unit],
        fallen: &[Unit],
        timings: Timings,
    ) -> Option<Self> {
        // HP as the combats go, starting from before the command.
        let mut hp: BTreeMap<UnitId, StatValue> = before.iter().map(|u| (u.id, u.hp)).collect();
        let fighter = |id: UnitId, hp: &BTreeMap<UnitId, StatValue>| {
            let unit = before.iter().find(|u| u.id == id)?;
            Some(Fighter {
                unit: id,
                name: unit.name.clone(),
                faction: unit.faction,
                start: hp.get(&id).copied().unwrap_or(unit.hp),
                max: unit.stats.hp,
            })
        };
        let mut bouts = Vec::new();
        let mut falls = Vec::new();
        for event in events {
            match event {
                Event::CombatResolved {
                    attacker,
                    defender,
                    outcome,
                    ..
                } => {
                    let (Some(a), Some(d)) = (fighter(*attacker, &hp), fighter(*defender, &hp))
                    else {
                        continue;
                    };
                    hp.insert(*attacker, outcome.attacker_hp);
                    hp.insert(*defender, outcome.defender_hp);
                    bouts.push(Bout {
                        attacker: a,
                        defender: d,
                        strikes: outcome.strikes.clone(),
                    });
                }
                Event::UnitFell { unit } => {
                    if let Some(u) = fallen.iter().find(|u| u.id == *unit) {
                        falls.push(u.clone());
                    }
                }
                _ => {}
            }
        }
        if bouts.is_empty() {
            return None;
        }
        let steps = timeline(&bouts, falls.len(), &timings);
        Some(Self {
            bouts,
            falls,
            steps,
            timings,
            t: 0.0,
            press: None,
        })
    }

    /// The combats, in order.
    pub fn bouts(&self) -> &[Bout] {
        &self.bouts
    }

    /// The units that fell, in order.
    pub fn falls(&self) -> &[Unit] {
        &self.falls
    }

    /// The timeline.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Seconds played.
    pub fn time(&self) -> f32 {
        self.t
    }

    /// The whole playback's length at normal speed.
    pub fn total(&self) -> f32 {
        self.steps.last().map_or(0.0, |s| s.start + s.len)
    }

    /// Whether it has played to the end.
    pub fn done(&self) -> bool {
        self.t >= self.total()
    }

    /// The step playing now and the seconds into it (the last step once
    /// done).
    pub fn now(&self) -> Option<(Step, f32)> {
        let step = self
            .steps
            .iter()
            .find(|s| self.t < s.start + s.len)
            .or_else(|| self.steps.last())?;
        Some((*step, (self.t - step.start).clamp(0.0, step.len)))
    }

    /// Confirm was pressed: while it stays held the playback runs fast; if
    /// it is released within [`Timings::tap`], it skips to the end.
    pub fn press(&mut self) {
        self.press.get_or_insert(0.0);
    }

    /// Advances the clock by `dt` seconds (`confirm_held`: Confirm is down
    /// this frame): [`Timings::fast`] times faster while held, to the end
    /// when a tap is released. A bad `dt` counts as 0.
    pub fn tick(&mut self, dt: f32, confirm_held: bool) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        if let Some(held) = self.press {
            if confirm_held {
                self.press = Some(held + dt);
            } else {
                self.press = None;
                if held < self.timings.tap {
                    self.t = self.total();
                    return;
                }
            }
        }
        let speed = if confirm_held { self.timings.fast } else { 1.0 };
        self.t = (self.t + dt * speed).min(self.total());
    }

    /// The combat the box shows now: the current one, or the last during
    /// the falls and the end.
    pub fn bout(&self) -> Option<&Bout> {
        let index = match self.now()?.0.beat {
            Beat::Intro { bout }
            | Beat::Flash { bout, .. }
            | Beat::Result { bout, .. }
            | Beat::Gap { bout, .. } => bout,
            Beat::Fall { .. } | Beat::Outro => self.bouts.len().checked_sub(1)?,
        };
        self.bouts.get(index)
    }

    /// Unit `id`'s HP as shown now, if it fights in this playback: its
    /// HP before the combats, then each strike's result, draining at
    /// [`Timings::drain_hp_per_s`] while the result is up.
    pub fn hp(&self, id: UnitId) -> Option<StatValue> {
        let mut shown = None;
        for (b, bout) in self.bouts.iter().enumerate() {
            for f in [&bout.attacker, &bout.defender] {
                if f.unit == id && shown.is_none() {
                    shown = Some(f.start);
                }
            }
            let target_of = |s: &Strike| match s.by {
                Side::Attacker => bout.defender.unit,
                Side::Defender => bout.attacker.unit,
            };
            for (i, strike) in bout.strikes.iter().enumerate() {
                if target_of(strike) != id {
                    continue;
                }
                let before = shown.unwrap_or(strike.target_hp_after);
                let Some(step) = self.step(Beat::Result { bout: b, strike: i }) else {
                    continue;
                };
                if self.t < step.start {
                    return shown;
                }
                let delta = strike.target_hp_after - before;
                let drained = (self.t - step.start) * self.timings.drain_hp_per_s;
                // HP values are small: exact in f32.
                #[allow(clippy::cast_precision_loss)]
                let whole = delta.abs() as f32;
                shown = Some(if drained >= whole {
                    strike.target_hp_after
                } else {
                    // 0 ≤ drained < |delta|, a small HP value: the cast
                    // is exact. The epsilon absorbs float error (0.2 s ×
                    // 30 HP/s is 6 HP, not 5.9999).
                    #[allow(clippy::cast_possible_truncation)]
                    let moved = (drained + 1e-3).floor() as StatValue;
                    before + delta.signum() * moved
                });
            }
        }
        shown
    }

    /// How far fallen unit `id` has faded (`0` = fully drawn, `1` = gone),
    /// if it falls in this playback.
    pub fn fade(&self, id: UnitId) -> Option<f32> {
        let fall = self.falls.iter().position(|u| u.id == id)?;
        let step = self.step(Beat::Fall { fall })?;
        Some(((self.t - step.start) / step.len.max(f32::EPSILON)).clamp(0.0, 1.0))
    }

    /// The message line: `<Name> has fallen.` from the start of a fall
    /// until the next one (or the end).
    pub fn message(&self) -> Option<String> {
        let (step, _) = self.now()?;
        let fall = match step.beat {
            Beat::Fall { fall } => fall,
            Beat::Outro => self.falls.len().checked_sub(1)?,
            _ => return None,
        };
        self.falls
            .get(fall)
            .map(|u| format!("{} has fallen.", u.name))
    }

    /// The step of `beat`, if the timeline has one.
    fn step(&self, beat: Beat) -> Option<Step> {
        self.steps.iter().find(|s| s.beat == beat).copied()
    }
}

/// Lays out the beats: per combat an intro, then per strike a flash, the
/// result (at least [`Timings::result`], longer while HP drains) and a gap
/// (none after the very last strike); then each fall; then the outro.
fn timeline(bouts: &[Bout], falls: usize, t: &Timings) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut at = 0.0;
    let mut push = |beat, len: f32| {
        steps.push(Step {
            beat,
            start: at,
            len,
        });
        at += len;
    };
    let last_bout = bouts.len().saturating_sub(1);
    for (b, bout) in bouts.iter().enumerate() {
        push(Beat::Intro { bout: b }, t.intro);
        let mut hp = [bout.attacker.start, bout.defender.start];
        let last_strike = bout.strikes.len().saturating_sub(1);
        for (i, s) in bout.strikes.iter().enumerate() {
            let target = match s.by {
                Side::Attacker => 1,
                Side::Defender => 0,
            };
            let delta = (s.target_hp_after - hp[target]).abs();
            hp[target] = s.target_hp_after;
            // HP values are small: exact in f32.
            #[allow(clippy::cast_precision_loss)]
            let drain = delta as f32 / t.drain_hp_per_s;
            push(Beat::Flash { bout: b, strike: i }, t.flash);
            push(Beat::Result { bout: b, strike: i }, t.result.max(drain));
            if !(b == last_bout && i == last_strike) {
                push(Beat::Gap { bout: b, strike: i }, t.gap);
            }
        }
    }
    for fall in 0..falls {
        push(Beat::Fall { fall }, t.fall);
    }
    push(Beat::Outro, t.outro);
    steps
}

/// What the box shows for strike `strike`: `HIT -7`, `MISS`, `CRITICAL!
/// -21`, or `+9` for an Absorb heal.
pub fn result_text(strike: &Strike) -> String {
    match (strike.hit, strike.crit, strike.healed) {
        (false, _, _) => "MISS".to_owned(),
        (true, _, true) => format!("HEAL +{}", strike.damage),
        (true, true, false) => format!("CRITICAL! -{}", strike.damage),
        (true, false, false) => format!("HIT -{}", strike.damage),
    }
}

/// The playback box's size, in cells.
pub const BOX_W: i32 = 44;

/// The playback box: centred at the top of the map view.
pub const BOX: Rect = Rect::new(
    MAP_VIEW.x + (MAP_VIEW.w - BOX_W) / 2,
    MAP_VIEW.y + 1,
    BOX_W,
    5,
);

/// Length of the box's HP bars, in cells.
pub const BOX_BAR_CELLS: i32 = 10;

/// Columns of the attacker's and the defender's text in the box.
pub const BOX_COLUMNS: [i32; 2] = [BOX.x + 2, BOX.x + 2 + BOX_W / 2];

/// Draws the playback box: both combatants' names (the striker's
/// flashing before its strike), HP and HP bars, and the current strike's
/// result under its target.
pub fn draw_box(buf: &mut GlyphBuffer, palette: &Palette, pb: &Playback) {
    let (Some(bout), Some((step, local))) = (pb.bout(), pb.now()) else {
        return;
    };
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    buf.fill_rect(BOX, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(BOX, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
    let flashing = match step.beat {
        Beat::Flash { strike, .. } => {
            // Whole half-periods since the flash began: on, off, on, …
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let phase = (local / pb.timings.blink.max(f32::EPSILON)) as u32;
            bout.strikes
                .get(strike)
                .filter(|_| phase.is_multiple_of(2))
                .map(|s| s.by)
        }
        _ => None,
    };
    let result = match step.beat {
        Beat::Result { strike, .. } | Beat::Gap { strike, .. } => bout.strikes.get(strike),
        _ => None,
    };
    let sides = [
        (Side::Attacker, &bout.attacker, BOX_COLUMNS[0]),
        (Side::Defender, &bout.defender, BOX_COLUMNS[1]),
    ];
    for (side, f, x) in sides {
        let color = c(faction_color(f.faction));
        let (fg, name_bg) = if flashing == Some(side) {
            (bg, color)
        } else {
            (color, bg)
        };
        buf.print(x, BOX.y + 1, &f.name, fg, name_bg);
        let hp = pb.hp(f.unit).unwrap_or(f.start);
        buf.print(x, BOX.y + 2, &format!("HP {hp:>2}"), c(UiColor::Text), bg);
        let (filled, bar) = hp_fill(hp, f.max, BOX_BAR_CELLS);
        for i in 0..BOX_BAR_CELLS {
            let (glyph, fg) = if i < filled {
                ('█', c(bar))
            } else {
                ('░', c(UiColor::TextDim))
            };
            buf.set(x + 6 + i, BOX.y + 2, Cell::new(glyph, fg, bg));
        }
        // The result shows under the strike's target.
        if let Some(s) = result.filter(|s| s.by != side) {
            buf.print(x, BOX.y + 3, &result_text(s), c(UiColor::TextHighlight), bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{CombatOutcome, Forecast, SideForecast};

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::quick_battle;

    fn strike(by: Side, hit: bool, crit: bool, damage: StatValue, after: StatValue) -> Strike {
        Strike {
            by,
            hit,
            crit,
            damage,
            healed: false,
            target_hp_after: after,
        }
    }

    fn numbers() -> SideForecast {
        SideForecast {
            damage: 8,
            followup_damage: 9,
            hit: 90,
            crit: 3,
            strikes: 2,
            effective: false,
            broken: false,
            affinity: None,
        }
    }

    /// The Quick Battle's units: lord 1 (19 HP), brigand 4 (20 HP), ...
    fn units() -> Vec<Unit> {
        quick_battle(&ctx().content).unwrap().units().to_vec()
    }

    /// The lord (19 HP) attacks brigand 4 (20 HP): hits for 8, the brigand
    /// misses, the lord crits for 27; the brigand falls.
    fn kill() -> (Vec<Event>, Vec<Unit>, Vec<Unit>) {
        let before = units();
        let strikes = vec![
            strike(Side::Attacker, true, false, 8, 12),
            strike(Side::Defender, false, false, 0, 19),
            strike(Side::Attacker, true, true, 27, 0),
        ];
        let events = vec![
            Event::UnitMoved {
                unit: UnitId(1),
                path: vec![],
            },
            Event::CombatResolved {
                attacker: UnitId(1),
                defender: UnitId(4),
                forecast: Forecast {
                    attacker: numbers(),
                    defender: Some(numbers()),
                },
                outcome: CombatOutcome {
                    strikes,
                    attacker_hp: 19,
                    defender_hp: 0,
                },
            },
            Event::UnitFell { unit: UnitId(4) },
            Event::UnitActed { unit: UnitId(1) },
        ];
        let mut fallen = before[3].clone();
        fallen.hp = 0;
        (events, before, vec![fallen])
    }

    fn playback() -> Playback {
        let (events, before, fallen) = kill();
        Playback::new(&events, &before, &fallen, TIMINGS).unwrap()
    }

    const T: Timings = TIMINGS;

    #[test]
    fn no_combat_no_playback() {
        let before = units();
        let wait = [Event::UnitActed { unit: UnitId(1) }];
        assert_eq!(Playback::new(&wait, &before, &[], TIMINGS), None);
    }

    #[test]
    fn the_timeline_lays_out_every_strike_then_the_fall() {
        let pb = playback();
        assert_eq!(pb.bouts().len(), 1);
        let bout = &pb.bouts()[0];
        let names = (bout.attacker.name.as_str(), bout.defender.name.as_str());
        assert_eq!(names, ("Test Lord", "Brigand"));
        assert_eq!((bout.attacker.start, bout.defender.start), (19, 20));
        let falls: Vec<UnitId> = pb.falls().iter().map(|u| u.id).collect();
        assert_eq!(falls, [UnitId(4)]);
        let beats: Vec<Beat> = pb.steps().iter().map(|s| s.beat).collect();
        let fl = |strike| Beat::Flash { bout: 0, strike };
        let re = |strike| Beat::Result { bout: 0, strike };
        let gap = |strike| Beat::Gap { bout: 0, strike };
        assert_eq!(
            beats,
            [
                Beat::Intro { bout: 0 },
                fl(0),
                re(0),
                gap(0),
                fl(1),
                re(1),
                gap(1),
                fl(2),
                re(2),
                Beat::Fall { fall: 0 },
                Beat::Outro,
            ]
        );
        // The crit drains 12 HP at 30 HP/s: 0.4 s, under the minimum.
        let lens: Vec<f32> = pb.steps().iter().map(|s| s.len).collect();
        assert_eq!(
            lens,
            [
                T.intro, T.flash, T.result, T.gap, T.flash, T.result, T.gap, T.flash, T.result,
                T.fall, T.outro
            ]
        );
        // Steps follow each other.
        for pair in pb.steps().windows(2) {
            assert!((pair[0].start + pair[0].len - pair[1].start).abs() < 1e-6);
        }
        let total: f32 = lens.iter().sum();
        assert!((pb.total() - total).abs() < 1e-5);
    }

    #[test]
    fn a_long_drain_lengthens_the_result() {
        let (mut events, before, fallen) = kill();
        let Event::CombatResolved { outcome, .. } = &mut events[1] else {
            unreachable!()
        };
        // One crit for all 20 HP: 20 / 30 s.
        outcome.strikes = vec![strike(Side::Attacker, true, true, 60, 0)];
        let pb = Playback::new(&events, &before, &fallen, TIMINGS).unwrap();
        assert!((pb.steps()[2].len - 20.0 / T.drain_hp_per_s).abs() < 1e-6);
    }

    /// The playback after frames of `dts` seconds with Confirm up.
    fn played(dts: &[f32]) -> Playback {
        let mut pb = playback();
        for &dt in dts {
            pb.tick(dt, false);
        }
        pb
    }

    #[test]
    fn positions_on_the_timeline_for_given_frame_times() {
        // The intro.
        let pb = played(&[0.1, 0.1]);
        assert!((pb.time() - 0.2).abs() < 1e-6);
        assert_eq!(pb.now().map(|(s, _)| s.beat), Some(Beat::Intro { bout: 0 }));
        assert_eq!((pb.hp(UnitId(1)), pb.hp(UnitId(4))), (Some(19), Some(20)));
        // The first flash, 0.1 s in.
        let pb = played(&[0.25, 0.15]);
        let (step, local) = pb.now().unwrap();
        assert_eq!(step.beat, Beat::Flash { bout: 0, strike: 0 });
        assert!((local - 0.1).abs() < 1e-5);
        // 0.2 s into the first result: 6 of its 8 HP drained.
        let pb = played(&[0.4, 0.4]);
        let beat = pb.now().unwrap().0.beat;
        assert_eq!(beat, Beat::Result { bout: 0, strike: 0 });
        assert_eq!(pb.hp(UnitId(4)), Some(14));
        // Its gap: the drain is over.
        let pb = played(&[1.0, 0.15]);
        assert_eq!(pb.now().unwrap().0.beat, Beat::Gap { bout: 0, strike: 0 });
        assert_eq!(pb.hp(UnitId(4)), Some(12));
        // The miss leaves the lord's HP alone.
        let pb = played(&[2.0]);
        assert_eq!(pb.hp(UnitId(1)), Some(19));
        // Bad frame times count as nothing.
        let pb = played(&[f32::NAN, -1.0, f32::INFINITY]);
        assert!(pb.time().abs() < f32::EPSILON);
        // Units not in the combat have no playback HP and no fade.
        assert_eq!(pb.hp(UnitId(2)), None);
        assert_eq!(pb.fade(UnitId(1)), None);
    }

    #[test]
    fn the_fall_fades_the_unit_and_shows_its_message_until_the_end() {
        let pb = playback();
        let fall = *pb
            .steps()
            .iter()
            .find(|s| s.beat == Beat::Fall { fall: 0 })
            .unwrap();
        let at = |t: f32| {
            let mut pb = playback();
            pb.tick(t, false);
            pb
        };
        let before = at(fall.start - 0.01);
        assert_eq!(
            (before.fade(UnitId(4)), before.message()),
            (Some(0.0), None)
        );
        assert_eq!(before.hp(UnitId(4)), Some(0));
        let mid = at(fall.start + fall.len / 2.0);
        assert!((mid.fade(UnitId(4)).unwrap() - 0.5).abs() < 1e-4);
        assert_eq!(mid.message().as_deref(), Some("Brigand has fallen."));
        let end = at(pb.total() - 0.01);
        assert_eq!(end.now().unwrap().0.beat, Beat::Outro);
        assert_eq!(end.fade(UnitId(4)), Some(1.0));
        assert_eq!(end.message().as_deref(), Some("Brigand has fallen."));
        assert!(!end.done());
        assert_eq!(end.bout().map(|b| b.defender.unit), Some(UnitId(4)));
        let done = at(pb.total() + 5.0);
        assert!(done.done());
        assert!(
            (done.time() - pb.total()).abs() < 1e-6,
            "clamped to the end"
        );
    }

    #[test]
    fn holding_confirm_is_four_times_as_fast_and_a_tap_skips() {
        // Held, pressed during the playback: four times as fast.
        let mut pb = playback();
        pb.press();
        pb.tick(0.1, true);
        pb.tick(0.1, true);
        assert!((pb.time() - 0.8).abs() < 1e-5);
        // Released after a long hold: no skip.
        pb.tick(0.1, false);
        assert!((pb.time() - 0.9).abs() < 1e-5);
        assert!(!pb.done());
        // Held without a press in the playback (still down from the
        // Confirm that attacked): fast, and letting go never skips.
        let mut pb = playback();
        pb.tick(0.1, true);
        pb.tick(0.1, false);
        assert!((pb.time() - 0.5).abs() < 1e-5);
        // A tap: pressed, released within the tap time.
        let mut pb = playback();
        pb.press();
        pb.tick(1.0 / 60.0, true);
        assert!(!pb.done());
        pb.tick(1.0 / 60.0, false);
        assert!(pb.done());
        // A second press while held doesn't restart the hold.
        let mut pb = playback();
        pb.press();
        pb.tick(T.tap, true);
        pb.press();
        pb.tick(0.01, false);
        assert!(!pb.done());
    }

    #[test]
    fn results_read_hit_miss_crit_and_heal() {
        let s = |hit, crit, healed| Strike {
            healed,
            ..strike(Side::Attacker, hit, crit, 7, 3)
        };
        assert_eq!(result_text(&s(true, false, false)), "HIT -7");
        assert_eq!(result_text(&s(false, false, false)), "MISS");
        assert_eq!(result_text(&s(true, true, false)), "CRITICAL! -7");
        assert_eq!(result_text(&s(true, false, true)), "HEAL +7");
    }

    #[test]
    fn a_second_combat_starts_from_the_hp_the_first_left() {
        let (mut events, before, fallen) = kill();
        let Event::CombatResolved { outcome, .. } = &mut events[1] else {
            unreachable!()
        };
        outcome.strikes.truncate(1);
        outcome.defender_hp = 12;
        let second = Event::CombatResolved {
            attacker: UnitId(1),
            defender: UnitId(4),
            forecast: Forecast {
                attacker: numbers(),
                defender: None,
            },
            outcome: CombatOutcome {
                strikes: vec![strike(Side::Attacker, true, false, 9, 3)],
                attacker_hp: 19,
                defender_hp: 3,
            },
        };
        events.insert(2, second);
        events.retain(|e| !matches!(e, Event::UnitFell { .. }));
        let mut pb = Playback::new(&events, &before, &fallen, TIMINGS).unwrap();
        assert_eq!(pb.bouts()[1].defender.start, 12);
        assert!(pb.falls().is_empty());
        pb.tick(pb.total(), false);
        assert_eq!(pb.hp(UnitId(4)), Some(3));
        assert_eq!(pb.message(), None);
    }

    #[test]
    fn the_box_flashes_the_striker_then_shows_the_result_under_the_target() {
        let c = ctx();
        let p = &c.palette;
        let render = |t: f32| {
            let mut pb = playback();
            pb.tick(t, false);
            let blank = Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black));
            let mut buf = GlyphBuffer::new(100, 32, blank);
            draw_box(&mut buf, p, &pb);
            buf
        };
        let (lx, rx) = (BOX_COLUMNS[0], BOX_COLUMNS[1]);
        let text = |buf: &GlyphBuffer, x: i32, y: i32, n: i32| -> String {
            (x..x + n).map(|x| buf.get(x, y).unwrap().glyph).collect()
        };
        // The first flash's first half-period: the lord's name inverted.
        let buf = render(T.intro + T.blink / 2.0);
        assert_eq!(text(&buf, lx, BOX.y + 1, 9), "Test Lord");
        assert_eq!(buf.get(lx, BOX.y + 1).unwrap().bg, p.get(UiColor::Player));
        // Its second half-period: normal.
        let buf = render(T.intro + T.blink * 1.5);
        assert_eq!(buf.get(lx, BOX.y + 1).unwrap().bg, p.get(UiColor::PanelBg));
        // The result under the brigand; the drain done by the gap.
        let buf = render(T.intro + T.flash + T.result + T.gap / 2.0);
        assert_eq!(text(&buf, rx, BOX.y + 3, 6), "HIT -8");
        assert_eq!(text(&buf, rx, BOX.y + 2, 5), "HP 12");
        assert_eq!(text(&buf, lx, BOX.y + 3, 6), "      ");
        // The brigand's miss shows under the lord.
        let second = T.intro + 2.0 * (T.flash + T.result + T.gap);
        let buf = render(second - T.gap / 2.0);
        assert_eq!(text(&buf, lx, BOX.y + 3, 4), "MISS");
        // At the start: the box, no result.
        let buf = render(0.0);
        assert_eq!(text(&buf, lx, BOX.y + 1, 9), "Test Lord");
        assert_eq!(text(&buf, lx, BOX.y + 3, 6), "      ");
    }
}
