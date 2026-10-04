//! `trpg-core`'s one integration-test program (ticket 0114). Cargo links
//! every file directly under `tests/` as its own program, so new integration
//! tests go here as a module: add `tests/it/<name>.rs` and a `mod <name>;`
//! line below. Never add a file directly under `tests/` (an `xtask` test
//! fails). Run one module with `cargo test -p trpg-core --test it <name>::`.

mod replay;
mod save_format;
