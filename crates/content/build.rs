//! Rebuild when anything under `assets/` changes: `include_dir!` embeds the
//! directory but does not tell cargo to watch it on stable Rust.

fn main() {
    println!("cargo:rerun-if-changed=../../assets");
}
