//! The one test that opts in to the bought art (ADR-0040). It only exists
//! with the `private-assets` feature (its `mod` line in `main.rs` is behind
//! it), which no gate turns on:
//!
//! ```text
//! cargo test -p trpg-content --features private-assets --test it private_assets::
//! ```
//!
//! Run it after changing anything in `assets-private/game/`; the Pages build
//! runs it before it builds the game.

/// Every content file still loads and validates with the private files laid
/// over `assets/`.
#[test]
fn the_content_loads_with_the_private_assets_over_it() {
    if let Err(errors) = trpg_content::load_embedded() {
        panic!("{errors}");
    }
}

/// Every portrait image that comes from the private assets is what
/// `cargo xtask portrait-import` writes (ADR-0043): a 64×64 cut bust, which
/// fills the dialogue frame at 4×, or a 48×48 face.
#[test]
fn bought_portraits_are_cut_busts_or_faces() {
    use trpg_content::bundle::{PRIVATE_DISPLAY_ROOT, display_path};

    let content = match trpg_content::load_embedded() {
        Ok(content) => content,
        Err(errors) => panic!("{errors}"),
    };
    for portrait in content.portraits.values() {
        for expression in &portrait.expressions {
            let shown = display_path(expression.image.path());
            let size = (expression.size.width, expression.size.height);
            assert!(
                !shown.starts_with(PRIVATE_DISPLAY_ROOT) || matches!(size, (64, 64) | (48, 48)),
                "{shown} is {size:?}"
            );
        }
    }
}
