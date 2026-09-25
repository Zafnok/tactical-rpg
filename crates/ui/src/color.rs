//! Colours: [`Rgb`] values and the [`Palette`] of named colours (ADR-0012).
//!
//! Code refers to colours by name, never raw RGB. The colours the UI itself
//! needs are the [`UiColor`] enum, so a missing name is impossible at render
//! time; data-driven names (terrain) go through [`Palette::lookup`] and are
//! validated when their content is loaded.

use std::collections::BTreeMap;

use trpg_content::PaletteDef;

/// A 24-bit RGB colour.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Builds a colour from its channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Linear interpolation from `self` (`t = 0`) to `other` (`t = 1`).
    /// `t` is clamped to `0..=1`; NaN counts as 0.
    #[must_use]
    pub fn lerp(self, other: Rgb, t: f32) -> Rgb {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let mix = |a: u8, b: u8| to_channel(f32::from(a) + (f32::from(b) - f32::from(a)) * t);
        Rgb::new(
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
        )
    }

    /// Multiplies every channel by `factor` (e.g. `0.5` dims by half),
    /// saturating at 0 and 255. NaN counts as 0.
    #[must_use]
    pub fn scale(self, factor: f32) -> Rgb {
        let f = if factor.is_nan() { 0.0 } else { factor };
        let s = |c: u8| to_channel(f32::from(c) * f);
        Rgb::new(s(self.r), s(self.g), s(self.b))
    }

    /// Lowercase `#rrggbb`.
    pub fn to_hex(self) -> String {
        trpg_content::palette::format_hex([self.r, self.g, self.b])
    }
}

impl From<[u8; 3]> for Rgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::new(r, g, b)
    }
}

/// Rounds and saturates a channel value to `0..=255`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
fn to_channel(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Declares [`UiColor`] with its palette names, in `REQUIRED_COLORS` order.
macro_rules! ui_colors {
    ($($variant:ident => $name:literal,)*) => {
        /// A colour the UI needs. Every variant's [`name`](UiColor::name) is a
        /// required palette colour (`trpg_content::palette::REQUIRED_COLORS`),
        /// so looking one up can't fail.
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
        pub enum UiColor {
            $(
                #[doc = concat!("Palette colour `", $name, "`.")]
                $variant,
            )*
        }

        impl UiColor {
            /// Every variant, in `REQUIRED_COLORS` order.
            pub const ALL: &[UiColor] = &[$(UiColor::$variant,)*];

            /// The colour's name in the palette file.
            pub const fn name(self) -> &'static str {
                match self {
                    $(UiColor::$variant => $name,)*
                }
            }
        }
    };
}

ui_colors! {
    Black => "black",
    White => "white",
    Text => "text",
    TextDim => "text_dim",
    TextHighlight => "text_highlight",
    PanelBg => "panel_bg",
    PanelBorder => "panel_border",
    PanelBorderFocus => "panel_border_focus",
    Player => "player",
    Enemy => "enemy",
    Ally => "ally",
    Neutral => "neutral",
    MoveRange => "move_range",
    AttackRange => "attack_range",
    HealRange => "heal_range",
    DangerZone => "danger_zone",
    Cursor => "cursor",
    HpHigh => "hp_high",
    HpMid => "hp_mid",
    HpLow => "hp_low",
    ExpBar => "exp_bar",
}

/// Named colours, built from the validated [`PaletteDef`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// One entry per [`UiColor`], indexed by `UiColor as usize`.
    ui: Vec<Rgb>,
    /// Every colour in the file, by name.
    named: BTreeMap<String, Rgb>,
}

impl Palette {
    /// Builds the palette. Fails with the names of any [`UiColor`]s the
    /// definition lacks (a palette loaded by `trpg_content` never does).
    pub fn new(def: &PaletteDef) -> Result<Self, Vec<&'static str>> {
        let named: BTreeMap<String, Rgb> = def
            .colors
            .iter()
            .map(|(name, &rgb)| (name.clone(), Rgb::from(rgb)))
            .collect();
        let mut ui = Vec::with_capacity(UiColor::ALL.len());
        let mut missing = Vec::new();
        for c in UiColor::ALL {
            match named.get(c.name()) {
                Some(&rgb) => ui.push(rgb),
                None => missing.push(c.name()),
            }
        }
        if missing.is_empty() {
            Ok(Self { ui, named })
        } else {
            Err(missing)
        }
    }

    /// The RGB value of a UI colour.
    pub fn get(&self, color: UiColor) -> Rgb {
        self.ui[color as usize]
    }

    /// Looks up a data-driven colour name (e.g. terrain).
    pub fn lookup(&self, name: &str) -> Option<Rgb> {
        self.named.get(name).copied()
    }

    /// Every named colour, alphabetically.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Rgb)> {
        self.named.iter().map(|(n, &c)| (n.as_str(), c))
    }

    /// The name of a colour with exactly this RGB value, if any. When several
    /// names share a value, [`UiColor`] names win (in [`UiColor::ALL`]
    /// order), then the alphabetically first other name.
    pub fn name_of(&self, rgb: Rgb) -> Option<&str> {
        UiColor::ALL
            .iter()
            .map(|c| c.name())
            .find(|&n| self.lookup(n) == Some(rgb))
            .or_else(|| {
                self.named
                    .iter()
                    .find(|&(_, &v)| v == rgb)
                    .map(|(n, _)| n.as_str())
            })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use trpg_content::palette::REQUIRED_COLORS;

    use super::*;

    /// The embedded game palette.
    pub(crate) fn game_palette() -> Palette {
        Palette::new(&PaletteDef::load().unwrap()).unwrap()
    }

    fn def(pairs: &[(&str, [u8; 3])]) -> PaletteDef {
        PaletteDef {
            colors: pairs.iter().map(|&(n, c)| (n.to_owned(), c)).collect(),
        }
    }

    #[test]
    fn ui_color_names_are_exactly_required_colors() {
        let names: Vec<&str> = UiColor::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(names, REQUIRED_COLORS);
    }

    #[test]
    fn ui_color_index_matches_all_order() {
        for (i, c) in UiColor::ALL.iter().enumerate() {
            assert_eq!(*c as usize, i);
        }
    }

    #[test]
    fn lerp_endpoints_and_midpoint() {
        let a = Rgb::new(0, 100, 255);
        let b = Rgb::new(200, 0, 55);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Rgb::new(100, 50, 155));
        assert_eq!(a.lerp(b, 0.25), Rgb::new(50, 75, 205));
    }

    #[test]
    fn lerp_clamps_t() {
        let a = Rgb::new(10, 20, 30);
        let b = Rgb::new(40, 50, 60);
        assert_eq!(a.lerp(b, -3.0), a);
        assert_eq!(a.lerp(b, 7.0), b);
        assert_eq!(a.lerp(b, f32::NAN), a);
    }

    #[test]
    fn scale_dims_and_saturates() {
        let c = Rgb::new(100, 200, 51);
        assert_eq!(c.scale(0.5), Rgb::new(50, 100, 26));
        assert_eq!(c.scale(1.0), c);
        assert_eq!(c.scale(2.0), Rgb::new(200, 255, 102));
        assert_eq!(c.scale(-1.0), Rgb::new(0, 0, 0));
        assert_eq!(c.scale(f32::NAN), Rgb::new(0, 0, 0));
    }

    #[test]
    fn to_hex_and_from_array() {
        assert_eq!(Rgb::from([1, 0xab, 255]).to_hex(), "#01abff");
    }

    #[test]
    fn palette_get_and_lookup() {
        let p = game_palette();
        let def = PaletteDef::load().unwrap();
        for c in UiColor::ALL {
            assert_eq!(p.get(*c), Rgb::from(def.get(c.name()).unwrap()));
        }
        assert_eq!(p.lookup("grass"), def.get("grass").map(Rgb::from));
        assert_eq!(p.lookup("no_such_colour"), None);
        let all: Vec<(&str, Rgb)> = p.iter().collect();
        assert_eq!(all.len(), def.colors.len());
        assert!(all.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(all.iter().all(|&(n, c)| p.lookup(n) == Some(c)));
    }

    #[test]
    fn palette_new_reports_missing_ui_colors() {
        assert_eq!(
            Palette::new(&PaletteDef::default()),
            Err(REQUIRED_COLORS.to_vec())
        );
        let mut d = PaletteDef::load().unwrap();
        d.colors.remove("cursor");
        d.colors.remove("text");
        assert_eq!(Palette::new(&d), Err(vec!["text", "cursor"]));
    }

    #[test]
    fn name_of_prefers_ui_names_then_alphabetical() {
        let mut pairs: Vec<(&str, [u8; 3])> =
            REQUIRED_COLORS.iter().map(|&n| (n, [9, 9, 9])).collect();
        pairs.push(("zeta", [1, 2, 3]));
        pairs.push(("alpha", [1, 2, 3]));
        pairs.push(("aaa", [9, 9, 9]));
        pairs.push(("text_dim", [4, 4, 4]));
        let p = Palette::new(&def(&pairs)).unwrap();
        assert_eq!(p.name_of(Rgb::new(9, 9, 9)), Some("black"));
        assert_eq!(p.name_of(Rgb::new(1, 2, 3)), Some("alpha"));
        assert_eq!(p.name_of(Rgb::new(4, 4, 4)), Some("text_dim"));
        assert_eq!(p.name_of(Rgb::new(7, 7, 7)), None);
    }
}
