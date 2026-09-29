//! `cargo xtask sfx`: renders the sounds we make ourselves (ticket 0213,
//! `docs/design/audio.md`) into `assets/audio/sfx/*.wav`.
//!
//! A small offline port of the Web Audio graph in
//! `assets-src/audio/sound-audition.html`, the page Nick picked the sounds
//! on. It has the page's building blocks (`tone`, `burst`, `pulse`, `env`,
//! `wet` and the master chain in `init()`) and each recipe keeps the page's
//! numbers. Where Web Audio's behaviour shapes the sound, this follows the
//! spec: `AudioParam` ramps, `PeriodicWave` normalisation, the
//! `BiquadFilterNode` formulas and `ConvolverNode` normalisation.
//!
//! Output is deterministic: noise and the reverb come from fixed seeds and
//! nothing reads the clock, so the same code always writes the same bytes.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use trpg_content::audio::SAMPLE_RATE;

/// Samples per second, as a float.
const SR: f64 = SAMPLE_RATE as f64;
/// Every sound is rendered into a buffer this long, then trimmed.
const RENDER_SECS: f64 = 4.0;
/// `init()`: the volume slider's default, 70 → 0.7^1.6.
const MASTER_GAIN: f64 = 0.565_293_720_402_445_7;
/// `init()`: `sfxBus.gain`.
const SFX_BUS_GAIN: f64 = 0.9;
/// `init()`: the reverb's return gain.
const REVERB_GAIN: f64 = 0.32;
/// `init()`: the reverb impulse is this long.
const REVERB_SECS: f64 = 1.8;
/// Loudest sample of the loudest sound after normalising: −1 dBFS.
const PEAK: f64 = 0.891_250_938_133_745_6;
/// Trailing samples quieter than −60 dBFS are trimmed.
const SILENCE: f64 = 0.001;
/// Fixed seeds, so the noise and the reverb are the same on every run.
const NOISE_SEED: u64 = 0x0213_0001;
const REVERB_SEED: u64 = 0x0213_0002;

/// A sound's recipe: what it plays into the page's buses.
type Recipe = fn() -> Mix;

/// The sounds, in the order written: file stem and recipe.
const SOUNDS: [(&str, Recipe); 5] = [
    ("menu_move", menu_move),
    ("menu_select", menu_select),
    ("menu_cancel", menu_cancel),
    ("miss", miss),
    ("heal", heal),
];

// ---------- recipes (the page's `MENU`, `SFX` and `HEAL`) ----------

/// `MENU.move` B: `chip('D4', t, 0.05, 0.015, 0.07, 1400)`.
fn menu_move() -> Mix {
    let mut mix = Mix::new();
    let p25 = Wave::pulse(0.25);
    mix.add(&chip(&p25, "D4", 0.0, 0.05, 0.015, 0.07, 1400.0));
    mix
}

/// `MENU.select` L: D4 then G4, 60 ms apart.
fn menu_select() -> Mix {
    let mut mix = Mix::new();
    let p25 = Wave::pulse(0.25);
    mix.add(&chip(&p25, "D4", 0.0, 0.04, 0.02, 0.1, 1600.0));
    mix.add(&chip(&p25, "G4", 0.06, 0.14, 0.04, 0.12, 1700.0));
    mix
}

/// `MENU.cancel` B: D4 then A3 on the square pulse, 70 ms apart.
fn menu_cancel() -> Mix {
    let mut mix = Mix::new();
    let p50 = Wave::pulse(0.5);
    mix.add(&chip(&p50, "D4", 0.0, 0.05, 0.03, 0.1, 1300.0));
    mix.add(&chip(&p50, "A3", 0.07, 0.1, 0.03, 0.1, 1300.0));
    mix
}

/// `SFX['miss:quick']` (W2): a band-passed noise whoosh sweeping up.
fn miss() -> Mix {
    let mut mix = Mix::new();
    let whoosh = Burst {
        t: 0.0,
        a: 0.07,
        d: 0.16,
        peak: 0.18,
        filter: Filter::Bandpass,
        f: 700.0,
        f2: Some(2600.0),
        q: 1.6,
    };
    mix.add(&whoosh.render(&mut Noise::new(NOISE_SEED)));
    mix
}

/// `HEAL.HE5`: `bloom(t, 0.3, 1.2)` plus a detuned G3 pair underneath, all
/// sent to the reverb at 0.5.
fn heal() -> Mix {
    let mut mix = Mix::new();
    let triangle = Wave::triangle();
    let (a, d) = (0.3, 1.2);
    for note in ["C4", "E4", "G4"] {
        for detune in [-6.0, 6.0] {
            let f = nf(note);
            let tone = Tone {
                f2: Some(f * 1.012),
                glide: Some(a + d * 0.6),
                a,
                d,
                peak: 0.05,
                detune,
                filter: Some(1200.0),
                ..Tone::new(&triangle, f, 0.0)
            };
            mix.add_wet(&tone.render(), 0.5);
        }
    }
    for detune in [-5.0, 5.0] {
        let tone = Tone {
            a,
            d,
            peak: 0.045,
            detune,
            filter: Some(900.0),
            ..Tone::new(&triangle, nf("G3"), 0.0)
        };
        mix.add_wet(&tone.render(), 0.5);
    }
    mix
}

/// The page's `chip(n, t, d, hold, peak, filt, w)`: a filtered pulse note.
fn chip(wave: &Wave, note: &str, t: f64, d: f64, hold: f64, peak: f64, filt: f64) -> Vec<f64> {
    Tone {
        d,
        hold,
        peak,
        filter: Some(filt),
        ..Tone::new(wave, nf(note), t)
    }
    .render()
}

/// The page's `nf`: a note name like `"D4"` or `"F#3"` → frequency in Hz.
fn nf(note: &str) -> f64 {
    let mut chars = note.chars();
    let semitone = match chars.next() {
        Some('C') => 0,
        Some('D') => 2,
        Some('E') => 4,
        Some('F') => 5,
        Some('G') => 7,
        Some('A') => 9,
        _ => 11,
    };
    let rest = chars.as_str();
    let (accidental, octave) = match rest.strip_prefix('#') {
        Some(o) => (1, o),
        None => rest.strip_prefix('b').map_or((0, rest), |o| (-1, o)),
    };
    let octave: i32 = octave.parse().unwrap_or(4);
    let midi = (octave + 1) * 12 + semitone + accidental;
    440.0 * 2f64.powf(f64::from(midi - 69) / 12.0)
}

// ---------- building blocks ----------

/// How an `AudioParam` event reaches its value from the previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ramp {
    /// `setValueAtTime`: jumps at its time.
    Set,
    /// `linearRampToValueAtTime`.
    Linear,
    /// `exponentialRampToValueAtTime`.
    Exp,
}

/// An `AudioParam` timeline: `(ramp, time, value)` events in time order,
/// read with Web Audio's rules.
#[derive(Debug, Clone)]
struct Param(Vec<(Ramp, f64, f64)>);

impl Param {
    fn constant(value: f64) -> Self {
        Self(vec![(Ramp::Set, 0.0, value)])
    }

    /// The value at time `t` (seconds).
    fn at(&self, t: f64) -> f64 {
        let mut prev = self.0[0];
        // Before the first event, its value.
        let t = t.max(prev.1);
        for &next in &self.0[1..] {
            if t < next.1 {
                let x = (t - prev.1) / (next.1 - prev.1);
                return match next.0 {
                    Ramp::Set => prev.2,
                    Ramp::Linear => prev.2 + (next.2 - prev.2) * x,
                    Ramp::Exp => prev.2 * (next.2 / prev.2).powf(x),
                };
            }
            prev = next;
        }
        prev.2
    }
}

/// The page's `env(g, t, a, peak, d, hold)`: a gain rising linearly from
/// silence to `peak` over `a`, holding, then falling exponentially over `d`.
fn env(t: f64, a: f64, peak: f64, d: f64, hold: f64) -> Param {
    // The page only sets the hold's end when `hold > 0`; with no hold that
    // event lands on the ramp's end with the same value, so it's harmless.
    Param(vec![
        (Ramp::Set, t, 0.0001),
        (Ramp::Linear, t + a, peak),
        (Ramp::Set, t + a + hold, peak),
        (Ramp::Exp, t + a + hold + d, 0.0001),
    ])
}

/// A periodic waveform as a Fourier series, like Web Audio's
/// `PeriodicWave`: `real[k]` and `imag[k]` are the cosine and sine
/// amplitudes of harmonic `k` (index 0, DC, is ignored), and `scale`
/// normalises the full-band wave's peak to 1.
#[derive(Debug, Clone)]
struct Wave {
    real: Vec<f64>,
    imag: Vec<f64>,
    scale: f64,
}

/// Points the peak is searched over when normalising a wave (Chrome's
/// wavetable size).
const WAVE_TABLE: u32 = 4096;

impl Wave {
    fn new(real: Vec<f64>, imag: Vec<f64>) -> Self {
        let mut wave = Self {
            real,
            imag,
            scale: 1.0,
        };
        let all = wave.real.len();
        let peak = (0..WAVE_TABLE)
            .map(|i| wave.sample(f64::from(i) / f64::from(WAVE_TABLE), all).abs())
            .fold(0.0, f64::max);
        wave.scale = 1.0 / peak;
        wave
    }

    /// Web Audio's built-in triangle: odd sine harmonics falling as 1/k²,
    /// alternating in sign (its 8/π² factor cancels in normalisation). 64
    /// terms, like `pulse`: Chrome's go on to 2048, but past the 63rd they
    /// are below −74 dB, and every triangle here is lowpassed far below
    /// them.
    fn triangle() -> Self {
        let n = 64;
        let mut imag = vec![0.0; n];
        for (k, b) in imag.iter_mut().enumerate().skip(1).step_by(2) {
            let sign = if k % 4 == 1 { 1.0 } else { -1.0 };
            let kf = to_f64(k);
            *b = sign / (kf * kf);
        }
        Self::new(vec![0.0; n], imag)
    }

    /// The page's `pulse(duty)`: 64 terms, harmonics 1..=63.
    fn pulse(duty: f64) -> Self {
        let n = 64;
        let (mut real, mut imag) = (vec![0.0; n], vec![0.0; n]);
        for k in 1..n {
            let kpi = to_f64(k) * PI;
            real[k] = (2.0 * kpi * duty).sin() / kpi;
            imag[k] = (2.0 / kpi) * (kpi * duty).sin().powi(2);
        }
        Self::new(real, imag)
    }

    /// The wave at `phase` (0..1 of a cycle), summing harmonics
    /// `1..=max_k`: the oscillator leaves out those at or above Nyquist,
    /// as Web Audio's band-limited tables do.
    fn sample(&self, phase: f64, max_k: usize) -> f64 {
        let (s1, c1) = (2.0 * PI * phase).sin_cos();
        let (mut s_prev, mut c_prev) = (0.0, 1.0);
        let (mut s, mut c) = (s1, c1);
        let mut sum = 0.0;
        for k in 1..self.real.len().min(max_k + 1) {
            sum += self.real[k] * c + self.imag[k] * s;
            // sin/cos of (k+1)θ from kθ and (k−1)θ.
            let next = (2.0 * c1 * s - s_prev, 2.0 * c1 * c - c_prev);
            (s_prev, c_prev) = (s, c);
            (s, c) = next;
        }
        sum * self.scale
    }
}

/// `BiquadFilterNode` types the recipes use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    Lowpass,
    Bandpass,
}

/// `input` through a `BiquadFilterNode`, coefficients recomputed every
/// sample from `freq` (so frequency ramps are sample-accurate). Formulas
/// from the Web Audio spec: `Q` is in dB for the lowpass, linear for the
/// bandpass.
fn biquad(filter: Filter, freq: &Param, q: f64, input: &[f64]) -> Vec<f64> {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    let mut out = Vec::with_capacity(input.len());
    for (i, &x) in input.iter().enumerate() {
        let w0 = 2.0 * PI * freq.at(time(i)) / SR;
        let (sin, cos) = w0.sin_cos();
        let [b0, b1, b2, a0, a1, a2] = match filter {
            Filter::Lowpass => {
                let alpha = sin / (2.0 * 10f64.powf(q / 20.0));
                let b = (1.0 - cos) / 2.0;
                [b, 1.0 - cos, b, 1.0 + alpha, -2.0 * cos, 1.0 - alpha]
            }
            Filter::Bandpass => {
                let alpha = sin / (2.0 * q);
                [alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha]
            }
        };
        let y = (b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
        (x2, x1, y2, y1) = (x1, x, y1, y);
        out.push(y);
    }
    out
}

/// Multiplies `signal` by a gain automation, sample by sample.
fn apply_gain(signal: &mut [f64], gain: &Param) {
    for (i, s) in signal.iter_mut().enumerate() {
        *s *= gain.at(time(i));
    }
}

/// The page's `tone({...})`: an oscillator, optionally lowpassed, through an
/// `env` gain. Field defaults (from [`Tone::new`]) are the page's.
#[derive(Debug, Clone)]
struct Tone<'w> {
    wave: &'w Wave,
    f: f64,
    /// Exponential glide target, reached `glide` seconds after `t` (the
    /// page's `glide ?? d`).
    f2: Option<f64>,
    glide: Option<f64>,
    t: f64,
    a: f64,
    d: f64,
    peak: f64,
    hold: f64,
    /// Cents.
    detune: f64,
    /// Lowpass cutoff, Hz.
    filter: Option<f64>,
    q: f64,
}

impl<'w> Tone<'w> {
    fn new(wave: &'w Wave, f: f64, t: f64) -> Self {
        Self {
            wave,
            f,
            f2: None,
            glide: None,
            t,
            a: 0.005,
            d: 0.2,
            peak: 0.3,
            hold: 0.0,
            detune: 0.0,
            filter: None,
            q: 0.7,
        }
    }

    fn render(&self) -> Vec<f64> {
        let mut freq = vec![(Ramp::Set, self.t, self.f)];
        if let Some(f2) = self.f2 {
            freq.push((Ramp::Exp, self.t + self.glide.unwrap_or(self.d), f2));
        }
        let freq = Param(freq);
        let ratio = 2f64.powf(self.detune / 1200.0);
        let stop = stop_time(self.t, self.a, self.hold, self.d);
        let mut out = vec![0.0; frames(RENDER_SECS)];
        let mut phase = 0.0;
        let (start, stop) = (frames(self.t), frames(stop));
        for (i, s) in out.iter_mut().enumerate().take(stop).skip(start) {
            let f = freq.at(time(i)) * ratio;
            *s = self.wave.sample(phase, floor_usize(SR / 2.0 / f));
            phase = (phase + f / SR).fract();
        }
        if let Some(cutoff) = self.filter {
            out = biquad(Filter::Lowpass, &Param::constant(cutoff), self.q, &out);
        }
        apply_gain(&mut out, &env(self.t, self.a, self.peak, self.d, self.hold));
        out
    }
}

/// When the page stops a source: 50 ms after its envelope ends.
fn stop_time(t: f64, a: f64, hold: f64, d: f64) -> f64 {
    t + a + hold + d + 0.05
}

/// A seeded random source (`SplitMix64`) standing in for `Math.random()`.
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    fn next(&mut self) -> f64 {
        let z = self.next_u64();
        #[allow(clippy::cast_precision_loss)] // 53 bits and 2^53: exact in an f64
        let unit = (z >> 11) as f64 / (1u64 << 53) as f64;
        unit
    }

    /// Uniform in `[-1, 1)`.
    fn signed(&mut self) -> f64 {
        self.next() * 2.0 - 1.0
    }
}

/// The page's `noise()`: 2 s of white noise, looped, plus the random start
/// offset each `burst` picks.
#[derive(Debug, Clone)]
struct Noise {
    buffer: Vec<f64>,
    rng: Rng,
}

impl Noise {
    fn new(seed: u64) -> Self {
        let mut rng = Rng(seed);
        let buffer = (0..frames(2.0)).map(|_| rng.signed()).collect();
        Self { buffer, rng }
    }

    /// The looped buffer playing from `start` to `stop` seconds, starting
    /// at a random point in its first second (`s.start(t, Math.random())`).
    fn play(&mut self, start: f64, stop: f64) -> Vec<f64> {
        let offset = frames(self.rng.next());
        let mut out = vec![0.0; frames(RENDER_SECS)];
        let first = frames(start);
        for (n, i) in (first..frames(stop)).enumerate() {
            out[i] = self.buffer[(offset + n) % self.buffer.len()];
        }
        out
    }
}

/// The page's `burst({...})`: filtered noise through an `env` gain, the
/// filter optionally sweeping to `f2` by the end of the decay.
#[derive(Debug, Clone)]
struct Burst {
    t: f64,
    a: f64,
    d: f64,
    peak: f64,
    filter: Filter,
    f: f64,
    f2: Option<f64>,
    q: f64,
}

impl Burst {
    fn render(&self, noise: &mut Noise) -> Vec<f64> {
        let source = noise.play(self.t, stop_time(self.t, self.a, 0.0, self.d));
        let mut freq = vec![(Ramp::Set, self.t, self.f)];
        if let Some(f2) = self.f2 {
            freq.push((Ramp::Exp, self.t + self.a + self.d, f2));
        }
        let mut out = biquad(self.filter, &Param(freq), self.q, &source);
        apply_gain(&mut out, &env(self.t, self.a, self.peak, self.d, 0.0));
        out
    }
}

/// The page's buses: what goes into `sfxBus`, and what `wet` sends to the
/// reverb (the send taps the tone before `sfxBus`, as on the page).
#[derive(Debug, Clone)]
struct Mix {
    sfx: Vec<f64>,
    send: Vec<f64>,
    wet: bool,
}

impl Mix {
    fn new() -> Self {
        Self {
            sfx: vec![0.0; frames(RENDER_SECS)],
            send: vec![0.0; frames(RENDER_SECS)],
            wet: false,
        }
    }

    /// Plays `signal` into `sfxBus`.
    fn add(&mut self, signal: &[f64]) {
        for (bus, s) in self.sfx.iter_mut().zip(signal) {
            *bus += s;
        }
    }

    /// The page's `wet(signal, amount)`: plays `signal` into `sfxBus` and
    /// sends it to the reverb at `amount`.
    fn add_wet(&mut self, signal: &[f64], amount: f64) {
        self.add(signal);
        self.wet = true;
        for (bus, s) in self.send.iter_mut().zip(signal) {
            *bus += s * amount;
        }
    }

    /// The master chain from `init()`: (sfxBus + reverb) × master gain,
    /// through the compressor.
    fn master(&self) -> Vec<f64> {
        let reverb = if self.wet {
            convolve(&self.send, &reverb_impulse())
        } else {
            vec![0.0; self.send.len()]
        };
        let mixed: Vec<f64> = self
            .sfx
            .iter()
            .zip(&reverb)
            .map(|(dry, wet)| (dry * SFX_BUS_GAIN + wet * REVERB_GAIN) * MASTER_GAIN)
            .collect();
        compress(&mixed)
    }
}

/// `init()`'s room reverb impulse: 1.8 s of noise decaying as (1 − t)^3.2,
/// scaled the way a `ConvolverNode` normalises its buffer (`normalize` is
/// true by default): 0.00125 / RMS. One channel: the page's stereo impulse
/// has two independent noise channels, and each speaker plays one of them.
fn reverb_impulse() -> Vec<f64> {
    let mut rng = Rng(REVERB_SEED);
    let len = frames(REVERB_SECS);
    let lenf = to_f64(len);
    let mut ir: Vec<f64> = (0..len)
        .map(|i| rng.signed() * (1.0 - to_f64(i) / lenf).powf(3.2))
        .collect();
    let rms = (ir.iter().map(|x| x * x).sum::<f64>() / lenf).sqrt();
    let scale = 0.00125 / rms.max(0.000_125);
    for x in &mut ir {
        *x *= scale;
    }
    ir
}

/// `signal` convolved with `ir`, truncated to `signal`'s length (via FFT).
fn convolve(signal: &[f64], ir: &[f64]) -> Vec<f64> {
    let n = (signal.len() + ir.len()).next_power_of_two();
    let complex = |x: &[f64]| {
        let mut v: Vec<(f64, f64)> = x.iter().map(|&r| (r, 0.0)).collect();
        v.resize(n, (0.0, 0.0));
        fft(&mut v, false);
        v
    };
    let (a, b) = (complex(signal), complex(ir));
    let mut product: Vec<(f64, f64)> = a
        .iter()
        .zip(&b)
        .map(|(&(ar, ai), &(br, bi))| (ar * br - ai * bi, ar * bi + ai * br))
        .collect();
    fft(&mut product, true);
    let scale = 1.0 / to_f64(n);
    product
        .iter()
        .take(signal.len())
        .map(|c| c.0 * scale)
        .collect()
}

/// In-place radix-2 FFT (`buf.len()` a power of two, at least 2), without
/// the inverse's 1/n scaling.
fn fft(buf: &mut [(f64, f64)], inverse: bool) {
    let n = buf.len();
    let shift = usize::BITS - n.trailing_zeros();
    let input = buf.to_vec();
    for (i, c) in buf.iter_mut().enumerate() {
        *c = input[i.reverse_bits() >> shift];
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        for start in (0..n).step_by(len) {
            for k in 0..half {
                let (wi, wr) = (sign * 2.0 * PI * to_f64(k) / to_f64(len)).sin_cos();
                let (ur, ui) = buf[start + k];
                let (xr, xi) = buf[start + k + half];
                let (vr, vi) = (xr * wr - xi * wi, xr * wi + xi * wr);
                buf[start + k] = (ur + vr, ui + vi);
                buf[start + k + half] = (ur - vr, ui - vi);
            }
        }
        len <<= 1;
    }
}

/// `init()`'s `DynamicsCompressor`: threshold −14 dB, ratio 4, and Web
/// Audio's defaults (knee 30 dB, attack 3 ms, release 250 ms). A simpler
/// curve than Chrome's: a quadratic soft knee starting at the threshold, a
/// peak detector, and no make-up gain or look-ahead (the output is
/// normalised afterwards, which does the make-up gain's job).
fn compress(input: &[f64]) -> Vec<f64> {
    const THRESHOLD: f64 = -14.0;
    const KNEE: f64 = 30.0;
    const RATIO: f64 = 4.0;
    let attack = (-1.0 / (0.003 * SR)).exp();
    let release = (-1.0 / (0.25 * SR)).exp();
    let mut reduction = 0.0; // dB, ≤ 0
    input
        .iter()
        .map(|&x| {
            let target = gain_reduction(20.0 * x.abs().max(1e-12).log10(), THRESHOLD, KNEE, RATIO);
            let coeff = if target.total_cmp(&reduction).is_lt() {
                attack
            } else {
                release
            };
            reduction = target + (reduction - target) * coeff;
            x * 10f64.powf(reduction / 20.0)
        })
        .collect()
}

/// The compressor's static curve: dB of gain change (≤ 0) at input level
/// `level` dB.
fn gain_reduction(level: f64, threshold: f64, knee: f64, ratio: f64) -> f64 {
    // Quadratic over the knee, then a straight line of slope 1/ratio − 1;
    // both are 0 below the threshold.
    let over = (level - threshold).max(0.0);
    let slope = 1.0 / ratio - 1.0;
    let in_knee = over.min(knee);
    slope * in_knee * in_knee / (2.0 * knee) + slope * (over - knee).max(0.0)
}

// ---------- rendering and files ----------

fn to_f64(n: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)] // sample counts are far below 2^52
    let x = n as f64;
    x
}

/// `x` rounded down to a count (0 for negatives).
fn floor_usize(x: f64) -> usize {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped, and small
    let n = x.max(0.0).floor() as usize;
    n
}

/// Sample frames in `secs` seconds (the first frame at or after that time).
fn frames(secs: f64) -> usize {
    floor_usize((secs * SR).round())
}

/// The time of frame `i`, seconds.
fn time(i: usize) -> f64 {
    to_f64(i) / SR
}

/// Renders every sound: normalised together (one gain for all, so they
/// keep the page's relative loudness), trailing silence trimmed, 16-bit.
pub fn render_all() -> Vec<(&'static str, Vec<i16>)> {
    let raw: Vec<(&str, Vec<f64>)> = SOUNDS
        .iter()
        .map(|&(name, recipe)| (name, recipe().master()))
        .collect();
    let loudest = raw
        .iter()
        .flat_map(|(_, s)| s.iter())
        .fold(0.0, |m: f64, x| m.max(x.abs()));
    let gain = PEAK / loudest;
    raw.into_iter()
        .map(|(name, samples)| {
            let scaled: Vec<f64> = samples.iter().map(|x| x * gain).collect();
            let end = scaled
                .iter()
                .rposition(|x| x.abs() >= SILENCE)
                .map_or(0, |i| i + 1);
            (name, scaled[..end].iter().map(|&x| to_i16(x)).collect())
        })
        .collect()
}

fn to_i16(x: f64) -> i16 {
    #[allow(clippy::cast_possible_truncation)] // clamped to the i16 range first
    let s = (x * 32767.0).round().clamp(-32768.0, 32767.0) as i16;
    s
}

/// A 44.1 kHz, 16-bit, mono PCM WAV file.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data_len = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // integer PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Where the files go, relative to the repo root.
pub const OUT_DIR: &str = "assets/audio/sfx";

/// Runs the command. Writes the WAVs into `<root>/assets/audio/sfx`, or with
/// `check`, only verifies the committed ones match what the code renders.
pub fn run(root: &Path, check: bool) -> Result<String, String> {
    let dir = root.join(OUT_DIR);
    let sounds = render_all();
    let summary = sounds
        .iter()
        .map(|(name, s)| format!("{name} {:.2} s", to_f64(s.len()) / SR))
        .collect::<Vec<_>>()
        .join(", ");
    if check {
        let stale: Vec<&str> = sounds
            .iter()
            .filter(|(name, samples)| {
                let committed = fs::read(dir.join(format!("{name}.wav"))).ok();
                !committed.is_some_and(|bytes| matches(&bytes, samples))
            })
            .map(|(name, _)| *name)
            .collect();
        if stale.is_empty() {
            return Ok(format!("sfx: up to date ({summary})"));
        }
        return Err(format!(
            "stale or missing in {OUT_DIR}: {}; rerun `cargo xtask sfx`",
            stale.join(", ")
        ));
    }
    fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    for (name, samples) in &sounds {
        let path = dir.join(format!("{name}.wav"));
        fs::write(&path, wav_bytes(samples))
            .map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(format!("sfx: wrote {OUT_DIR}: {summary}"))
}

/// Whether a committed file holds `samples`. The header must match exactly;
/// samples may differ by 1 step, since `sin`/`exp` can round differently
/// on another OS's maths library.
fn matches(bytes: &[u8], samples: &[i16]) -> bool {
    let expected = wav_bytes(samples);
    bytes.len() == expected.len()
        && bytes[..44] == expected[..44]
        && bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .zip(samples)
            .all(|(&got, &want)| {
                let got = i16::from_le_bytes(got);
                (i32::from(got) - i32::from(want)).abs() <= 1
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_root;

    fn secs(samples: &[i16]) -> f64 {
        to_f64(samples.len()) / SR
    }

    fn rms(x: &[f64]) -> f64 {
        (x.iter().map(|v| v * v).sum::<f64>() / to_f64(x.len())).sqrt()
    }

    fn sine(freq: f64, secs: f64) -> Vec<f64> {
        (0..frames(secs))
            .map(|i| (2.0 * PI * freq * time(i)).sin())
            .collect()
    }

    #[test]
    fn note_names_match_the_page() {
        assert!((nf("A4") - 440.0).abs() < 1e-9);
        assert!((nf("D4") - 293.664_767_9).abs() < 1e-6);
        assert!((nf("G3") - 195.997_717_9).abs() < 1e-6);
        assert!((nf("C#4") - nf("Db4")).abs() < 1e-9);
        assert!((nf("B3") * 2.0 - nf("B4")).abs() < 1e-9);
        assert!((nf("F4") - 349.228_231_4).abs() < 1e-6);
        assert!((nf("E4") - 329.627_556_9).abs() < 1e-6);
        assert!((nf("C4") - 261.625_565_3).abs() < 1e-6);
    }

    #[test]
    fn pulse_harmonics_match_the_page_formula() {
        for duty in [0.125, 0.25, 0.5] {
            let wave = Wave::pulse(duty);
            assert_eq!(wave.real.len(), 64);
            assert_eq!((wave.real[0], wave.imag[0]), (0.0, 0.0));
            for k in 1..64 {
                let kpi = to_f64(k) * PI;
                let re = (2.0 * kpi * duty).sin() / kpi;
                let im = 2.0 / kpi * (kpi * duty).sin().powi(2);
                assert!((wave.real[k] - re).abs() < 1e-15, "duty {duty} k {k}");
                assert!((wave.imag[k] - im).abs() < 1e-15, "duty {duty} k {k}");
            }
        }
    }

    #[test]
    fn waves_are_normalised_to_a_peak_of_one() {
        for wave in [Wave::pulse(0.25), Wave::pulse(0.5), Wave::triangle()] {
            let peak = (0..4096)
                .map(|i| wave.sample(f64::from(i) / 4096.0, usize::MAX - 1).abs())
                .fold(0.0, f64::max);
            assert!((peak - 1.0).abs() < 1e-9, "{peak}");
        }
        // A quarter-duty pulse is high for about a quarter of the cycle.
        let p25 = Wave::pulse(0.25);
        let high = (0..400)
            .filter(|&i| p25.sample(f64::from(i) / 400.0, 63) > 0.0)
            .count();
        assert!((90..=110).contains(&high), "{high}");
        // The triangle peaks at a quarter cycle.
        let tri = Wave::triangle();
        assert!((tri.sample(0.25, 4096) - 1.0).abs() < 1e-3);
        assert!(tri.sample(0.0, 4096).abs() < 1e-3);
    }

    #[test]
    fn oscillators_drop_harmonics_above_nyquist() {
        let wave = Wave::pulse(0.5);
        // Only the fundamental: a pure sine.
        let s = wave.sample(0.25, 1) / wave.scale;
        assert!((s - wave.imag[1]).abs() < 1e-12);
        assert!((wave.sample(0.3, 1) - wave.sample(0.3, 2)).abs() < 1e-12); // k=2 is 0
        assert!((wave.sample(0.3, 1) - wave.sample(0.3, 3)).abs() > 1e-3);
    }

    #[test]
    fn tones_leave_out_harmonics_at_or_above_nyquist() {
        // A 10 kHz square: its 3rd harmonic (30 kHz) is above Nyquist, so
        // the tone is a pure sine at the fundamental's level.
        let square = Wave::pulse(0.5);
        let tone = Tone {
            a: 0.001,
            hold: 0.1,
            peak: 1.0,
            ..Tone::new(&square, 10_000.0, 0.0)
        };
        let out = tone.render();
        let level = square.imag[1] * square.scale;
        for i in frames(0.01)..frames(0.05) {
            let want = level * (2.0 * PI * 10_000.0 * time(i)).sin();
            assert!((out[i] - want).abs() < 1e-6, "{i}: {} vs {want}", out[i]);
        }
    }

    #[test]
    fn env_reaches_its_peak_at_t_plus_a() {
        let (t, a, peak, d) = (0.1, 0.02, 0.5, 0.3);
        let e = env(t, a, peak, d, 0.0);
        assert!((e.at(t) - 0.0001).abs() < 1e-12);
        assert!((e.at(t + a) - peak).abs() < 1e-12);
        assert!((e.at(t + a / 2.0) - f64::midpoint(0.0001, peak)).abs() < 1e-12);
        let loudest = (0..1000)
            .map(|i| f64::from(i) / 1000.0)
            .max_by(|x, y| e.at(*x).total_cmp(&e.at(*y)))
            .unwrap();
        assert!((loudest - (t + a)).abs() < 1e-9, "{loudest}");
        // Exponential decay: halfway through, the geometric mean.
        let mid = e.at(t + a + d / 2.0);
        assert!((mid - (peak * 0.0001f64).sqrt()).abs() < 1e-9, "{mid}");
        assert!((e.at(t + a + d) - 0.0001).abs() < 1e-12);
        assert!((e.at(5.0) - 0.0001).abs() < 1e-12);
        assert!((e.at(0.0) - 0.0001).abs() < 1e-12);
    }

    #[test]
    fn env_holds_the_peak() {
        let e = env(0.0, 0.01, 0.2, 0.1, 0.05);
        assert!((e.at(0.01) - 0.2).abs() < 1e-12);
        assert!((e.at(0.04) - 0.2).abs() < 1e-12);
        assert!((e.at(0.0599) - 0.2).abs() < 1e-12);
        assert!(e.at(0.07) < 0.2 * 0.5);
    }

    #[test]
    fn params_ramp_like_web_audio() {
        let p = Param(vec![
            (Ramp::Set, 1.0, 100.0),
            (Ramp::Exp, 2.0, 400.0),
            (Ramp::Linear, 3.0, 0.0),
            (Ramp::Set, 4.0, 7.0),
        ]);
        assert!((p.at(0.5) - 100.0).abs() < 1e-9);
        assert!((p.at(1.5) - 200.0).abs() < 1e-9);
        assert!((p.at(2.5) - 200.0).abs() < 1e-9);
        assert!((p.at(3.5) - 0.0).abs() < 1e-9);
        assert!((p.at(4.5) - 7.0).abs() < 1e-9);
        // An event's own time already has its value.
        assert!((p.at(4.0) - 7.0).abs() < 1e-9);
        assert!((p.at(2.0) - 400.0).abs() < 1e-9);
    }

    #[test]
    fn lowpass_attenuates_a_tone_above_cutoff() {
        let cutoff = Param::constant(1000.0);
        let low = biquad(Filter::Lowpass, &cutoff, 0.7, &sine(200.0, 0.5));
        let high = biquad(Filter::Lowpass, &cutoff, 0.7, &sine(8000.0, 0.5));
        let tail = frames(0.1);
        assert!((rms(&low[tail..]) - 0.5f64.sqrt()).abs() < 0.03);
        assert!(rms(&high[tail..]) < 0.02, "{}", rms(&high[tail..]));
    }

    #[test]
    fn lowpass_q_is_in_decibels() {
        // At the cutoff, Web Audio's lowpass gain is exactly Q dB.
        let out = biquad(
            Filter::Lowpass,
            &Param::constant(1000.0),
            6.0,
            &sine(1000.0, 0.5),
        );
        let gain_db = 20.0 * (rms(&out[frames(0.1)..]) / 0.5f64.sqrt()).log10();
        assert!((gain_db - 6.0).abs() < 0.1, "{gain_db}");
    }

    #[test]
    fn bandpass_passes_its_centre_only() {
        let centre = Param::constant(2000.0);
        let pass = biquad(Filter::Bandpass, &centre, 1.6, &sine(2000.0, 0.5));
        let below = biquad(Filter::Bandpass, &centre, 1.6, &sine(200.0, 0.5));
        let above = biquad(Filter::Bandpass, &centre, 1.6, &sine(15000.0, 0.5));
        let tail = frames(0.1);
        assert!((rms(&pass[tail..]) - 0.5f64.sqrt()).abs() < 0.03);
        assert!(rms(&below[tail..]) < 0.1);
        assert!(rms(&above[tail..]) < 0.15);
    }

    #[test]
    fn tones_glide_and_detune() {
        let sine = Wave::new(vec![0.0, 0.0], vec![0.0, 1.0]);
        let crossings = |x: &[f64]| x.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        let plain = Tone {
            a: 0.001,
            d: 1.0,
            hold: 0.5,
            ..Tone::new(&sine, 100.0, 0.0)
        };
        // 100 Hz for the first half second: ~50 cycles.
        let base = crossings(&plain.render()[..frames(0.5)]);
        assert!((49..=51).contains(&base), "{base}");
        let up = Tone {
            detune: 1200.0,
            ..plain.clone()
        };
        let octave = crossings(&up.render()[..frames(0.5)]);
        assert!((99..=101).contains(&octave), "{octave}");
        let glide = Tone {
            f2: Some(400.0),
            glide: Some(0.5),
            ..plain.clone()
        };
        let late = crossings(&glide.render()[frames(0.5)..frames(1.0)]);
        assert!((199..=201).contains(&late), "{late}");
        // Without `glide`, the glide lasts `d`.
        let slow = Tone {
            f2: Some(400.0),
            ..plain.clone()
        };
        let mid = crossings(&slow.render()[frames(0.5)..frames(1.0)]);
        assert!((110..=190).contains(&mid), "{mid}");
        // Silent before its start and after its stop.
        let later = Tone { t: 0.2, ..plain }.render();
        assert!(later[..frames(0.2)].iter().all(|x| x.abs() < 1e-12));
        assert!(later[frames(1.8)..].iter().all(|x| x.abs() < 1e-12));
    }

    #[test]
    fn convolution_matches_the_direct_sum() {
        let signal = [1.0, 2.0, 0.0, -1.0, 0.5];
        let ir = [0.5, -0.25, 0.125];
        let got = convolve(&signal, &ir);
        let mut want = [0.0; 5];
        for (n, w) in want.iter_mut().enumerate() {
            for (k, h) in ir.iter().enumerate() {
                if k <= n {
                    *w += h * signal[n - k];
                }
            }
        }
        assert_eq!(got.len(), 5);
        for (g, w) in got.iter().zip(want) {
            assert!((g - w).abs() < 1e-12, "{got:?} vs {want:?}");
        }
    }

    #[test]
    fn reverb_impulse_is_normalised_and_decays() {
        let ir = reverb_impulse();
        assert_eq!(ir.len(), frames(1.8));
        assert!((rms(&ir) - 0.00125).abs() < 1e-9);
        let quarter = ir.len() / 4;
        assert!(rms(&ir[..quarter]) > 10.0 * rms(&ir[3 * quarter..]));
    }

    #[test]
    fn rng_is_splitmix64() {
        // The reference sequence for seed 0.
        let mut rng = Rng(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        let mut rng = Rng(0);
        let unit = rng.next();
        // The top 53 bits of that first output, as a fraction.
        assert!((unit - 0.883_310_808_213_642_6).abs() < 1e-15, "{unit}");
    }

    #[test]
    fn sources_stop_50_ms_after_their_envelope() {
        assert!((stop_time(0.5, 0.07, 0.02, 0.16) - 0.8).abs() < 1e-12);
        assert!((stop_time(0.0, 0.07, 0.0, 0.16) - 0.28).abs() < 1e-12);
    }

    #[test]
    fn only_wet_signals_reach_the_reverb() {
        let signal = vec![0.5; 10];
        let mut mix = Mix::new();
        mix.add(&signal);
        assert!(!mix.wet);
        assert!(mix.send.iter().all(|x| *x == 0.0));
        assert!((mix.sfx[9] - 0.5).abs() < 1e-12);
        mix.add_wet(&signal, 0.25);
        assert!(mix.wet);
        assert!((mix.sfx[9] - 1.0).abs() < 1e-12);
        assert!((mix.send[9] - 0.125).abs() < 1e-12);
        assert!(mix.send[10].abs() < 1e-12);
    }

    #[test]
    fn noise_is_seeded_and_loops() {
        let mut a = Noise::new(1);
        let b = Noise::new(1);
        assert_eq!(a.buffer, b.buffer);
        assert_ne!(a.buffer, Noise::new(2).buffer);
        assert_eq!(a.buffer.len(), frames(2.0));
        assert!(a.buffer.iter().all(|x| (-1.0..1.0).contains(x)));
        assert!(rms(&a.buffer) > 0.55 && rms(&a.buffer) < 0.6); // uniform: 1/√3
        let played = a.play(0.1, 3.0);
        assert!(played[..frames(0.1)].iter().all(|x| *x == 0.0));
        let start = frames(0.1);
        assert_eq!(
            played[start].to_bits(),
            played[start + a.buffer.len()].to_bits()
        );
        assert!((played[start] - played[start + 1]).abs() > 0.0);
        assert!(played[frames(3.0)..].iter().all(|x| *x == 0.0));
    }

    #[test]
    fn compressor_leaves_quiet_sounds_alone_and_squashes_loud_ones() {
        let quiet: Vec<f64> = sine(440.0, 0.5).iter().map(|x| x * 0.05).collect();
        assert_eq!(compress(&quiet), quiet);
        // +20 dB peaks: 34 dB over the threshold, about 14 dB of reduction.
        let loud: Vec<f64> = sine(440.0, 0.5).iter().map(|x| x * 10.0).collect();
        let out = compress(&loud);
        let ratio = rms(&out[frames(0.3)..]) / rms(&loud[frames(0.3)..]);
        assert!((0.15..0.3).contains(&ratio), "{ratio}");
        assert!(gain_reduction(-14.0, -14.0, 30.0, 4.0).abs() < 1e-12);
        assert!((gain_reduction(26.0, -14.0, 30.0, 4.0) + 0.75 * 25.0).abs() < 1e-9);
        assert!((gain_reduction(1.0, -14.0, 30.0, 4.0) + 0.75 * 225.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn compressor_attacks_in_3_ms_and_releases_in_250_ms() {
        // +20 dB for 0.1 s, then quiet.
        let loud = 0.1;
        let signal: Vec<f64> = (0..frames(1.0))
            .map(|i| if i < frames(loud) { 10.0 } else { 0.001 })
            .collect();
        let out = compress(&signal);
        let target = gain_reduction(20.0, -14.0, 30.0, 4.0);
        let db = |i: usize| 20.0 * (out[i] / signal[i]).log10();
        // One time constant into the attack: 1 − 1/e of the way there.
        let attack = frames(0.003) - 1;
        let want = target * (1.0 - (-to_f64(attack + 1) / (0.003 * SR)).exp());
        assert!((db(attack) - want).abs() < 0.01, "{} vs {want}", db(attack));
        assert!((db(frames(loud) - 1) - target).abs() < 1e-6);
        // One time constant into the release: 1/e of the reduction left.
        let release = frames(loud) + frames(0.25) - 1;
        let want = target * (-to_f64(frames(0.25)) / (0.25 * SR)).exp();
        assert!(
            (db(release) - want).abs() < 0.01,
            "{} vs {want}",
            db(release)
        );
    }

    #[test]
    fn wav_header_is_16_bit_mono_44k1() {
        let bytes = wav_bytes(&[1, -2]);
        assert_eq!(bytes.len(), 48);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[4..8], &40u32.to_le_bytes());
        assert_eq!(&bytes[40..44], &4u32.to_le_bytes());
        assert_eq!(&bytes[44..], &[1, 0, 0xFE, 0xFF]);
        assert!(trpg_content::audio::format_problem("x.wav", &bytes).is_none());
        assert!(matches(&bytes, &[2, -3]));
        assert!(!matches(&bytes, &[3, -2]));
        assert!(!matches(&bytes, &[1, -2, 0]));
    }

    #[test]
    fn sounds_are_as_long_as_their_recipes() {
        let sounds = render_all();
        let len = |name: &str| secs(&sounds.iter().find(|(n, _)| *n == name).unwrap().1);
        // a + hold + d of the last note, from the start; within 50 ms.
        for (name, recipe) in [
            ("menu_move", 0.005 + 0.015 + 0.05),
            ("menu_select", 0.06 + 0.005 + 0.04 + 0.14),
            ("menu_cancel", 0.07 + 0.005 + 0.03 + 0.1),
            ("miss", 0.07 + 0.16),
        ] {
            assert!((len(name) - recipe).abs() <= 0.05, "{name}: {}", len(name));
        }
        assert!(len("menu_move") < 0.15);
        // heal: 0.3 + 1.2 s, then the reverb tail down to −60 dB.
        let heal = len("heal");
        assert!((1.5..=1.5 + REVERB_SECS).contains(&heal), "{heal}");
    }

    #[test]
    fn sounds_keep_the_pages_relative_loudness() {
        let sounds = render_all();
        let peak = |name: &str| {
            let s = &sounds.iter().find(|(n, _)| *n == name).unwrap().1;
            s.iter().map(|x| i32::from(*x).abs()).max().unwrap()
        };
        let loudest = sounds.iter().map(|(n, _)| peak(n)).max().unwrap();
        assert_eq!(loudest, i32::from(to_i16(PEAK)));
        // On the page, the menu chips are much quieter than the heal chord.
        assert!(peak("menu_move") < peak("heal") / 2);
        // Trimmed: the last sample is at the −60 dB line.
        for (name, s) in &sounds {
            let last = i32::from(*s.last().unwrap()).abs();
            assert!((32..=40).contains(&last), "{name}: {last}");
        }
    }

    #[test]
    fn rendering_twice_writes_identical_files() {
        let dir = std::env::temp_dir().join(format!("xtask-sfx-{}", std::process::id()));
        let (a, b) = (dir.join("a"), dir.join("b"));
        for root in [&a, &b] {
            let msg = run(root, false).unwrap();
            assert!(msg.starts_with("sfx: wrote "), "{msg}");
        }
        for (name, _) in SOUNDS {
            let file = format!("{OUT_DIR}/{name}.wav");
            let first = fs::read(a.join(&file)).unwrap();
            assert!(first.len() > 44);
            assert_eq!(first, fs::read(b.join(&file)).unwrap(), "{name}");
        }
        assert!(run(&a, true).unwrap().starts_with("sfx: up to date"));
        fs::write(a.join(OUT_DIR).join("miss.wav"), b"junk").unwrap();
        let err = run(&a, true).unwrap_err();
        assert!(err.contains("miss") && !err.contains("heal"), "{err}");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The committed WAVs must be what the code renders.
    #[test]
    fn committed_sounds_are_up_to_date() {
        run(&repo_root(), true).unwrap();
    }
}
