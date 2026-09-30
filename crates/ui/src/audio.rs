//! Audio as data (ADR-0026): screens ask for sounds and music through
//! [`Ctx::audio`](crate::Ctx::audio); [`Game`](crate::Game) hands the
//! requests, and the [`MusicState`]'s commands, to `app` in each
//! [`FrameOutput`](crate::FrameOutput). Nothing here plays anything, so
//! screens stay testable in the headless `Harness`.
//!
//! ```
//! # use trpg_ui::audio::{AudioQueue, AudioRequest};
//! let mut audio = AudioQueue::default();
//! audio.play_sound("menu_move");
//! audio.play_music("title");
//! assert_eq!(audio.pending()[1], AudioRequest::PlayMusic { cue: "title".into() });
//! ```

/// Something a screen wants heard. Cue ids are the ones in
/// `assets/audio/audio.ron`.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioRequest {
    /// Play a sound cue once, at `volume` (0–1) times the cue's own volume.
    PlaySound {
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Switch to a music cue. Asking for the track already playing does
    /// nothing.
    PlayMusic {
        /// The music cue.
        cue: String,
    },
    /// Fade the music out.
    StopMusic,
}

impl AudioRequest {
    /// The cue this request names, if any.
    pub fn cue(&self) -> Option<&str> {
        match self {
            Self::PlaySound { cue, .. } | Self::PlayMusic { cue } => Some(cue),
            Self::StopMusic => None,
        }
    }
}

/// The requests screens made this frame. `Game` empties it every frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioQueue {
    requests: Vec<AudioRequest>,
}

impl AudioQueue {
    /// Plays sound `cue` once at its own volume.
    pub fn play_sound(&mut self, cue: &str) {
        self.play_sound_at(cue, 1.0);
    }

    /// Plays sound `cue` once at `volume` (0–1, clamped) times its own
    /// volume.
    pub fn play_sound_at(&mut self, cue: &str, volume: f32) {
        let volume = if volume.is_nan() {
            0.0
        } else {
            volume.clamp(0.0, 1.0)
        };
        self.requests.push(AudioRequest::PlaySound {
            cue: cue.to_owned(),
            volume,
        });
    }

    /// Switches the music to `cue` (nothing happens if it's already on).
    pub fn play_music(&mut self, cue: &str) {
        self.requests.push(AudioRequest::PlayMusic {
            cue: cue.to_owned(),
        });
    }

    /// Fades the music out.
    pub fn stop_music(&mut self) {
        self.requests.push(AudioRequest::StopMusic);
    }

    /// The requests made since the last [`take`](Self::take), in order.
    pub fn pending(&self) -> &[AudioRequest] {
        &self.requests
    }

    /// Empties the queue, returning its requests in order.
    pub fn take(&mut self) -> Vec<AudioRequest> {
        std::mem::take(&mut self.requests)
    }
}

/// The menu sounds (`docs/design/audio.md`): the same three cues mean the
/// same thing on every screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSound {
    /// The highlight moved (`menu_move`).
    Move,
    /// Something was confirmed or opened (`menu_select`).
    Select,
    /// Something was backed out of or closed (`menu_cancel`).
    Cancel,
    /// Confirm on something that can't be chosen, e.g. a greyed-out item.
    /// Nick wants its own warning tone (ticket 0427); until then it is
    /// `menu_cancel`.
    Denied,
}

impl MenuSound {
    /// The sound cue.
    pub const fn cue(self) -> &'static str {
        match self {
            Self::Move => "menu_move",
            Self::Select => "menu_select",
            Self::Cancel | Self::Denied => "menu_cancel",
        }
    }
}

/// The map cursor's tick, once per tile it moves (the manifest plays it
/// quieter than `menu_move`).
pub const CURSOR_MOVE: &str = "cursor_move";

impl AudioQueue {
    /// Plays a menu sound.
    pub fn menu(&mut self, sound: MenuSound) {
        self.play_sound(sound.cue());
    }
}

/// The music cue pool `pool` yields for `seed`: the same seed always gives
/// the same cue. `None` if the manifest has no such pool or it is empty.
/// This is not core's simulation RNG (ADR-0019), so a pick never changes a
/// battle or its replay.
pub fn pick_from_pool<'a>(
    manifest: &'a trpg_content::AudioManifest,
    pool: &str,
    seed: u64,
) -> Option<&'a str> {
    let cues = manifest.pools.get(pool)?;
    let len = u64::try_from(cues.len()).ok().filter(|&n| n > 0)?;
    let index = usize::try_from(splitmix64(seed) % len).ok()?;
    cues.get(index).map(String::as_str)
}

/// One step of splitmix64: spreads nearby seeds (e.g. a counter) over the
/// whole range.
fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// What `app` must do to the music this frame. A track is loaded, started,
/// has its volume changed and is stopped, which also unloads it.
#[derive(Debug, Clone, PartialEq)]
pub enum MusicCommand {
    /// Start loading `cue`'s file; it will be started soon.
    Load {
        /// The music cue.
        cue: String,
    },
    /// Play `cue` (loaded earlier) from the start at full gain, looped if
    /// the manifest says so. If it isn't loaded yet, play it once it is.
    Start {
        /// The music cue.
        cue: String,
    },
    /// Set `cue`'s gain (0–1, a multiplier on its manifest volume).
    Gain {
        /// The music cue.
        cue: String,
        /// The new gain.
        gain: f32,
    },
    /// Stop `cue` and free it (also cancels a load).
    Stop {
        /// The music cue.
        cue: String,
    },
}

/// Which track plays, and the fade between tracks (`audio.md`: the old one
/// fades out, then the new one starts; a cue already playing doesn't
/// restart). Pure: feed it requests and time, read the commands.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicState {
    /// Fade-out length in seconds.
    fade_secs: f32,
    /// The track playing (or fading out).
    current: Option<String>,
    /// The fade in progress, if any.
    fade: Option<Fade>,
}

/// A fade-out of the current track.
#[derive(Debug, Clone, PartialEq)]
struct Fade {
    /// Seconds since it began.
    elapsed: f32,
    /// What starts when it ends (already loading); `None` for silence.
    next: Option<String>,
}

impl MusicState {
    /// Silence, with fades `fade_secs` long (negative or NaN counts as 0).
    pub fn new(fade_secs: f32) -> Self {
        Self {
            fade_secs: fade_secs.max(0.0),
            current: None,
            fade: None,
        }
    }

    /// The track playing or fading out.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// The track that plays once the fade ends, or the current one.
    pub fn target(&self) -> Option<&str> {
        match &self.fade {
            Some(fade) => fade.next.as_deref(),
            None => self.current(),
        }
    }

    /// Applies one request (sound requests are ignored), appending the
    /// commands it needs to `out`.
    pub fn request(&mut self, request: &AudioRequest, out: &mut Vec<MusicCommand>) {
        match request {
            AudioRequest::PlaySound { .. } => {}
            AudioRequest::PlayMusic { cue } => self.play(cue, out),
            AudioRequest::StopMusic => self.stop(out),
        }
    }

    fn play(&mut self, cue: &str, out: &mut Vec<MusicCommand>) {
        let Some(current) = &self.current else {
            out.push(MusicCommand::Load { cue: cue.into() });
            out.push(MusicCommand::Start { cue: cue.into() });
            self.current = Some(cue.into());
            return;
        };
        if self.target() == Some(cue) {
            return;
        }
        if let Some(Fade {
            next: Some(next), ..
        }) = &self.fade
        {
            // A newer request wins: drop the one waiting to start.
            out.push(MusicCommand::Stop { cue: next.clone() });
        }
        if current == cue {
            // Asked again for the track that is fading out: keep it.
            out.push(MusicCommand::Gain {
                cue: cue.into(),
                gain: 1.0,
            });
            self.fade = None;
            return;
        }
        out.push(MusicCommand::Load { cue: cue.into() });
        let elapsed = self.fade.as_ref().map_or(0.0, |f| f.elapsed);
        self.fade = Some(Fade {
            elapsed,
            next: Some(cue.into()),
        });
    }

    fn stop(&mut self, out: &mut Vec<MusicCommand>) {
        if self.current.is_none() {
            return;
        }
        let elapsed = match self.fade.take() {
            Some(Fade { elapsed, next }) => {
                if let Some(next) = next {
                    out.push(MusicCommand::Stop { cue: next });
                }
                elapsed
            }
            None => 0.0,
        };
        self.fade = Some(Fade {
            elapsed,
            next: None,
        });
    }

    /// Advances a fade by `dt` seconds, appending the gain change, or the
    /// stop and next start when it ends.
    pub fn update(&mut self, dt: f32, out: &mut Vec<MusicCommand>) {
        let (Some(current), Some(fade)) = (&self.current, &mut self.fade) else {
            return;
        };
        if dt.is_finite() {
            fade.elapsed += dt.max(0.0);
        }
        if fade.elapsed < self.fade_secs {
            out.push(MusicCommand::Gain {
                cue: current.clone(),
                gain: 1.0 - fade.elapsed / self.fade_secs,
            });
            return;
        }
        out.push(MusicCommand::Stop {
            cue: current.clone(),
        });
        let next = fade.next.take();
        if let Some(next) = &next {
            out.push(MusicCommand::Start { cue: next.clone() });
        }
        self.current = next;
        self.fade = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(cue: &str) -> AudioRequest {
        AudioRequest::PlayMusic { cue: cue.into() }
    }

    fn load(cue: &str) -> MusicCommand {
        MusicCommand::Load { cue: cue.into() }
    }

    fn start(cue: &str) -> MusicCommand {
        MusicCommand::Start { cue: cue.into() }
    }

    fn stop(cue: &str) -> MusicCommand {
        MusicCommand::Stop { cue: cue.into() }
    }

    fn gain(cue: &str, gain: f32) -> MusicCommand {
        MusicCommand::Gain {
            cue: cue.into(),
            gain,
        }
    }

    /// Applies `requests` then advances `dt`, returning the commands.
    fn frame(m: &mut MusicState, requests: &[AudioRequest], dt: f32) -> Vec<MusicCommand> {
        let mut out = Vec::new();
        for r in requests {
            m.request(r, &mut out);
        }
        m.update(dt, &mut out);
        out
    }

    /// Music with a 0.5 s fade, playing `cue`.
    fn playing(cue: &str) -> MusicState {
        let mut m = MusicState::new(0.5);
        frame(&mut m, &[play(cue)], 0.0);
        m
    }

    #[test]
    fn the_first_track_starts_at_once() {
        let mut m = MusicState::new(0.5);
        assert_eq!(m.current(), None);
        assert_eq!(
            frame(&mut m, &[play("title")], 0.1),
            [load("title"), start("title")]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("title"));
        assert_eq!(frame(&mut m, &[], 0.1), []);
    }

    #[test]
    fn the_same_cue_does_not_restart() {
        let mut m = playing("title");
        assert_eq!(frame(&mut m, &[play("title"), play("title")], 0.1), []);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn a_switch_fades_out_then_starts_the_new_track() {
        let mut m = playing("title");
        assert_eq!(
            frame(&mut m, &[play("battle")], 0.0),
            [load("battle"), gain("title", 1.0)]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("battle"));
        assert_eq!(frame(&mut m, &[], 0.25), [gain("title", 0.5)]);
        assert_eq!(frame(&mut m, &[], 0.125), [gain("title", 0.25)]);
        // Asking again for the incoming track changes nothing.
        assert_eq!(frame(&mut m, &[play("battle")], 0.0), [gain("title", 0.25)]);
        assert_eq!(frame(&mut m, &[], 0.125), [stop("title"), start("battle")]);
        assert_eq!(m.current(), Some("battle"));
        assert_eq!(frame(&mut m, &[], 0.1), []);
    }

    #[test]
    fn stop_music_fades_out_to_silence() {
        let mut m = playing("title");
        let stop_req = AudioRequest::StopMusic;
        assert_eq!(
            frame(&mut m, std::slice::from_ref(&stop_req), 0.25),
            [gain("title", 0.5)]
        );
        assert_eq!(m.target(), None);
        assert_eq!(
            frame(&mut m, std::slice::from_ref(&stop_req), 0.0),
            [gain("title", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.3), [stop("title")]);
        assert_eq!(m.current(), None);
        // Stopping silence does nothing.
        assert_eq!(frame(&mut m, &[stop_req], 1.0), []);
    }

    #[test]
    fn a_new_request_during_a_fade_wins() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        // The waiting track is dropped; the fade keeps its progress.
        assert_eq!(
            frame(&mut m, &[play("boss")], 0.0),
            [stop("battle"), load("boss"), gain("title", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.25), [stop("title"), start("boss")]);
        assert_eq!(m.current(), Some("boss"));
        // A stop during a switch drops the waiting track too.
        frame(&mut m, &[play("town")], 0.25);
        assert_eq!(
            frame(&mut m, &[AudioRequest::StopMusic], 0.0),
            [stop("town"), gain("boss", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.25), [stop("boss")]);
        assert_eq!(m.current(), None);
        // And a play during a fade to silence switches to it.
        let mut m = playing("title");
        frame(&mut m, &[AudioRequest::StopMusic], 0.25);
        assert_eq!(
            frame(&mut m, &[play("battle")], 0.25),
            [load("battle"), stop("title"), start("battle")]
        );
    }

    #[test]
    fn asking_for_the_fading_track_keeps_it() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        assert_eq!(
            frame(&mut m, &[play("title")], 1.0),
            [stop("battle"), gain("title", 1.0)]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("title"));
        let mut m = playing("title");
        frame(&mut m, &[AudioRequest::StopMusic], 0.25);
        assert_eq!(frame(&mut m, &[play("title")], 1.0), [gain("title", 1.0)]);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn a_zero_fade_switches_in_the_same_frame() {
        for secs in [0.0, -1.0, f32::NAN] {
            let mut m = MusicState::new(secs);
            frame(&mut m, &[play("title")], 0.0);
            assert_eq!(
                frame(&mut m, &[play("battle")], 0.0),
                [load("battle"), stop("title"), start("battle")]
            );
        }
    }

    #[test]
    fn bad_frame_times_do_not_advance_a_fade() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        for dt in [-1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(frame(&mut m, &[], dt), [gain("title", 0.5)]);
        }
    }

    #[test]
    fn sound_requests_leave_the_music_alone() {
        let mut m = playing("title");
        let sound = AudioRequest::PlaySound {
            cue: "menu_move".into(),
            volume: 1.0,
        };
        assert_eq!(frame(&mut m, &[sound], 0.1), []);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn the_queue_records_requests_in_order() {
        let mut q = AudioQueue::default();
        q.play_sound("a");
        q.play_sound_at("b", 0.6);
        q.play_sound_at("c", 2.0);
        q.play_sound_at("d", -1.0);
        q.play_sound_at("e", f32::NAN);
        q.play_music("m");
        q.stop_music();
        let sound = |cue: &str, volume| AudioRequest::PlaySound {
            cue: cue.into(),
            volume,
        };
        let expected = [
            sound("a", 1.0),
            sound("b", 0.6),
            sound("c", 1.0),
            sound("d", 0.0),
            sound("e", 0.0),
            play("m"),
            AudioRequest::StopMusic,
        ];
        assert_eq!(q.pending(), expected);
        assert_eq!(q.take(), expected);
        assert!(q.pending().is_empty());
        assert_eq!(
            expected.map(|r| r.cue().map(str::to_owned))[4..],
            [Some("e".to_owned()), Some("m".to_owned()), None]
        );
    }

    /// The embedded audio manifest (with the real `skirmish` pool).
    fn manifest() -> trpg_content::AudioManifest {
        trpg_content::load_embedded().unwrap().audio
    }

    #[test]
    fn a_pool_pick_is_a_cue_of_that_pool_and_fixed_by_the_seed() {
        let m = manifest();
        let pool = &m.pools["skirmish"];
        for seed in [0, 1, 2, u64::MAX] {
            let cue = pick_from_pool(&m, "skirmish", seed).unwrap();
            assert!(pool.iter().any(|c| c == cue), "{cue}");
            assert_eq!(pick_from_pool(&m, "skirmish", seed), Some(cue));
        }
    }

    /// The published splitmix64 outputs for state 0 (the first two steps).
    #[test]
    fn splitmix64_matches_the_reference() {
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(0x9E37_79B9_7F4A_7C15), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn an_unknown_or_empty_pool_picks_nothing() {
        let mut m = manifest();
        assert_eq!(pick_from_pool(&m, "no_such_pool", 1), None);
        m.pools.insert("empty".into(), Vec::new());
        assert_eq!(pick_from_pool(&m, "empty", 1), None);
    }

    proptest::proptest! {
        // Each case loads the embedded content; a few dozen are plenty.
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]

        /// Over enough seeds, every track in the pool comes up.
        #[test]
        fn every_track_in_a_pool_can_be_picked(start in proptest::prelude::any::<u64>()) {
            let m = manifest();
            let pool = &m.pools["skirmish"];
            let picked: std::collections::BTreeSet<&str> = (0..200)
                .filter_map(|i| pick_from_pool(&m, "skirmish", start.wrapping_add(i)))
                .collect();
            proptest::prop_assert_eq!(picked.len(), pool.len());
        }
    }
}
