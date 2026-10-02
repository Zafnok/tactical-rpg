//! Rebuild when anything under `assets/` changes: `include_dir!` embeds the
//! directory but does not tell cargo to watch it on stable Rust. The same
//! for `assets-private/game/` when the `private-assets` feature embeds it
//! (ADR-0040), after checking that it is there.

use std::path::Path;

/// The bought art, relative to this crate (ADR-0040).
const PRIVATE: &str = "../../assets-private/game";

fn main() {
    println!("cargo:rerun-if-changed=../../assets");
    if std::env::var_os("CARGO_FEATURE_PRIVATE_ASSETS").is_some() {
        assert!(
            Path::new(PRIVATE).is_dir(),
            "the `private-assets` feature needs the bought art in assets-private/game/, \
             which isn't there. Fetch it with `cargo xtask private-assets` (it needs \
             access to the private repository), or build without the feature to use \
             the public placeholders."
        );
        println!("cargo:rerun-if-changed={PRIVATE}");
    }
}
