//! `trpg-ui`'s one integration-test program (ticket 0114). Cargo links every
//! file directly under `tests/` as its own program, so new integration tests
//! go here as a module: add `tests/it/<name>.rs` and a `mod <name>;` line
//! below. Never add a file directly under `tests/` (an `xtask` test fails).
//! Run one module with `cargo test -p trpg-ui --test it <name>::`.

mod battle;
mod class_change;
mod controller;
mod credits;
mod dialogue;
mod flow;
mod key_bindings;
mod key_bindings_screen;
mod layout_picker;
mod map_skin;
mod options;
mod save;
mod split_keys;
mod sprite_test;
mod title;
