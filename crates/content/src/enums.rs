//! Serde mirrors of `trpg-core` enums used in data files. `core` has no serde
//! dependency (ADR-0004), so each enum is mirrored here with the same variant
//! names and converted with `From`. An unknown variant name is a RON parse
//! error with a line and column.

use serde::Deserialize;

/// Declares a deserializable mirror of a core enum and its conversion.
macro_rules! mirror {
    ($raw:ident => $core:ty { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
        pub(crate) enum $raw {
            $($variant),+
        }

        impl From<$raw> for $core {
            fn from(raw: $raw) -> Self {
                match raw {
                    $($raw::$variant => <$core>::$variant),+
                }
            }
        }
    };
}

mirror!(RawStatKind => trpg_core::StatKind { Hp, Str, Mag, Dex, Spd, Def, Res, Mov });
mirror!(RawWeaponKind => trpg_core::WeaponKind { Sword, Spear, Axe, Bow, Gauntlet });
mirror!(RawWeaponRank => trpg_core::WeaponRank { E, D, C, B, A, S });
mirror!(RawArmourWeight => trpg_core::ArmourWeight { Light, Medium, Heavy });
mirror!(RawUnitTag => trpg_core::UnitTag { Mounted, Flying, Armored });
mirror!(RawElement => trpg_core::Element { Fire, Ice, None });
mirror!(RawAffinity => trpg_core::Affinity { Weak, Resist, Absorb });

#[cfg(test)]
mod tests {
    use trpg_core::{StatKind, WeaponRank};

    use super::*;
    use crate::ron_loader::parse_ron;

    #[test]
    fn names_match_core_variants() {
        let stats: Vec<RawStatKind> =
            parse_ron("t.ron", "[Hp, Str, Mag, Dex, Spd, Def, Res, Mov]").unwrap_or_default();
        let stats: Vec<StatKind> = stats.into_iter().map(StatKind::from).collect();
        assert_eq!(stats, StatKind::ALL);
        let ranks: Vec<RawWeaponRank> =
            parse_ron("t.ron", "[E, D, C, B, A, S]").unwrap_or_default();
        let ranks: Vec<WeaponRank> = ranks.into_iter().map(WeaponRank::from).collect();
        assert!(ranks.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(ranks.len(), 6);
    }

    #[test]
    fn unknown_name_is_a_positioned_error() {
        let err = parse_ron::<Vec<RawWeaponKind>>("t.ron", "[\n  Sword,\n  Whip,\n]").err();
        assert_eq!(err.as_ref().and_then(|e| e.line), Some(3));
        assert!(err.is_some_and(|e| e.message.contains("Whip")));
    }
}
