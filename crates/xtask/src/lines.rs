//! `cargo xtask lines [scene]`: lists every dialogue line with its line id
//! (ADR-0045 §3), for the translation and voice tools and for humans.

use trpg_content::DialogueTable;

pub const USAGE: &str = "usage: cargo xtask lines [scene]\n\n\
prints one row per dialogue line (speech, narration, reply), tab-separated:\n  \
<line id>  <speaker>  <text>\n\
the speaker is a character id, \">\" for narration or \"*\" for a reply.\n\
with a scene id, only that scene's lines; otherwise every scene's, by scene id.";

/// The rows to print: `id<TAB>speaker<TAB>text` for every line of `scene`,
/// or of every scene (ordered by scene id) when `None`. Lines are in
/// script order within a scene.
pub fn rows(dialogue: &DialogueTable, scene: Option<&str>) -> Result<Vec<String>, String> {
    let scenes: Vec<_> = match scene {
        Some(id) => vec![
            dialogue
                .get(id)
                .ok_or_else(|| format!("no scene \"{id}\" in assets/dialogue/"))?,
        ],
        None => dialogue.scenes.values().collect(),
    };
    Ok(scenes
        .iter()
        .flat_map(|s| s.lines())
        .map(|l| format!("{}\t{}\t{}", l.id, l.speaker, l.text))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dialogue() -> DialogueTable {
        let source = "@scene b\n> Hello.\n@end\n@scene a\n@left test_lord neutral\n\
                      test_lord: Hello.\n@choice\n* wry: Hello.\n  > Hello.\n* blunt: No.\n\
                      @endchoice\n@end\n";
        trpg_content::dialogue::from_sources(&[("t.dlg", source)], None, None, None, None).unwrap()
    }

    #[test]
    fn one_row_per_line_of_a_scene_in_script_order() {
        assert_eq!(
            rows(&dialogue(), Some("a")).unwrap(),
            [
                "a_9e73f1df\ttest_lord\tHello.",
                "a_bc2c818f\t*\tHello.",
                "a_10c29f83\t>\tHello.",
                "a_b3c0740c\t*\tNo.",
            ]
        );
    }

    #[test]
    fn without_a_scene_every_scene_is_listed_by_id() {
        let rows = rows(&dialogue(), None).unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0], "a_9e73f1df\ttest_lord\tHello.");
        assert_eq!(rows[4], "b_10c29f83\t>\tHello.");
    }

    #[test]
    fn an_unknown_scene_is_an_error() {
        assert_eq!(
            rows(&dialogue(), Some("c")),
            Err("no scene \"c\" in assets/dialogue/".to_owned())
        );
    }
}
