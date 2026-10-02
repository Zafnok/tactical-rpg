//! The one test that opts in to the bought art (ADR-0040). It only exists
//! with the `private-assets` feature, which no gate turns on:
//!
//! ```text
//! cargo test -p trpg-content --features private-assets --test private_assets
//! ```
//!
//! Run it after changing anything in `assets-private/game/`; the Pages build
//! runs it before it builds the game.
#![cfg(feature = "private-assets")]

/// Every content file still loads and validates with the private files laid
/// over `assets/`.
#[test]
fn the_content_loads_with_the_private_assets_over_it() {
    if let Err(errors) = trpg_content::load_embedded() {
        panic!("{errors}");
    }
}
