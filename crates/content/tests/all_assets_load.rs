//! ADR-0005 rule 3: the whole embedded asset bundle must load and validate.

#[test]
fn all_embedded_assets_load() {
    if let Err(errors) = trpg_content::load_embedded() {
        panic!("{errors}");
    }
}
