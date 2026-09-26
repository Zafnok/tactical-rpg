//! `quad-storage`/`sapp-jsutils` (web storage, ticket 0207) are miniquad JS
//! plugins: their `extern "C"` functions are implemented in JS and loaded
//! into the page at runtime, not linked in. Without `--allow-undefined`,
//! rust-lld treats them as link errors instead of leaving them as wasm
//! imports. Set here (via `cargo:rustc-link-arg`, additive) rather than in
//! `.cargo/config.toml`'s `rustflags`: a `RUSTFLAGS` env var (as CI sets,
//! for `-D warnings`) replaces config-file `rustflags` wholesale rather
//! than merging with it, which would silently drop this flag.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--allow-undefined");
    }
}
