//! ADR-0005 rule 3: the whole embedded asset bundle must load and validate.

#[test]
fn all_embedded_assets_load() {
    match trpg_content::load_embedded() {
        Err(errors) => panic!("{errors}"),
        // The sprite map skin's test tileset is among them (ticket 0433).
        Ok(content) => assert!(content.tilesets.contains_key("test")),
    }
}
