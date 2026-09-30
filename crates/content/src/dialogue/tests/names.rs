//! Tests of name tokens (`{n:<id>}`) and names written out (0709).

use super::*;

#[test]
fn known_name_tokens_pass() {
    let src = scene(
        "> {n:king} rides for {n:place.thornmarch}. {N:place.thornmarch} is cold.
@caption {N:place.harrowby}, dusk
test_lord: {n:faction.crown}, {lead}.",
    );
    assert_eq!(errors(&src), Vec::<String>::new());
}

#[test]
fn unknown_name_ids() {
    let src = scene("> {n:kingg} and {n:} and {N:King}.\n@caption {n:place.nowhere}");
    let hint = "name ids are listed in assets/data/names.ron";
    assert_eq!(
        errors(&src),
        [
            format!("t.dlg:4: unknown name id \"kingg\" in {{n:kingg}}; {hint}"),
            format!("t.dlg:4: unknown name id \"\" in {{n:}}; {hint}"),
            format!("t.dlg:4: unknown name id \"King\" in {{N:King}}; {hint}"),
            format!("t.dlg:5: unknown name id \"place.nowhere\" in {{n:place.nowhere}}; {hint}"),
        ]
    );
}

#[test]
fn without_a_names_table_only_the_id_syntax_is_checked() {
    let src = scene("> {n:anything} and {n:Bad}.");
    let errs: Vec<String> = from_sources(&[("t.dlg", src.as_str())], None, None, None)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errs,
        [
            "t.dlg:4: unknown name id \"Bad\" in {n:Bad}; name ids are listed in assets/data/names.ron"
        ]
    );
}

#[test]
fn the_lead_is_not_a_name_token() {
    assert_eq!(
        errors(&scene("> {n:lead} waits.")),
        [
            "t.dlg:4: {n:lead} is only the lead's default name; write {lead} for the name the \
             player chose"
        ]
    );
}

#[test]
fn unterminated_name_token() {
    assert_eq!(
        errors(&scene("> Ride for {n:king.")),
        ["t.dlg:4: \"{\" has no closing \"}\""]
    );
}

#[test]
fn names_written_out_are_errors() {
    let src = scene(
        "> Emeric rides.
@caption Harrowby, dusk
test_lord: Back to Thornmarch.
test_knight: For the Crown of Ardeval!
@choice
* a: Aske can shoot.
  > Fine.
* b: No.
  > Fine.
@endchoice",
    );
    assert_eq!(
        errors(&src),
        [
            "t.dlg:4: \"Emeric\" is written out; write {n:king} (\"Emeric\") so a rename reaches \
             this line",
            "t.dlg:5: \"Harrowby\" is written out; write {n:place.harrowby} (\"Harrowby\") so a \
             rename reaches this line",
            "t.dlg:6: \"Thornmarch\" is written out; write {n:place.thornmarch} (\"the \
             Thornmarch\") so a rename reaches this line",
            "t.dlg:7: \"Crown of Ardeval\" is written out; write {n:faction.crown} (\"the Crown \
             of Ardeval\") so a rename reaches this line",
            "t.dlg:9: \"Aske\" is written out; write {n:poacher} (\"Aske\") so a rename reaches \
             this line",
        ]
    );
}

#[test]
fn the_lead_default_name_written_out() {
    assert_eq!(
        errors(&scene("test_knight: Ellery, wait.")),
        [
            "t.dlg:4: \"Ellery\" is the lead's default name; write {lead} for the name the \
             player chose"
        ]
    );
}

#[test]
fn only_whole_capitalised_names_in_text_count() {
    // Comments, ordinary words, other case and parts of words aren't hits.
    let src = scene(
        "# Emeric's scene: Aske and Rue.
> You'll rue this. I swore a vow; breath fails.
> Emerics, Askew, harrowby.",
    );
    assert_eq!(errors(&src), Vec::<String>::new());
}

#[test]
fn name_tokens_count_as_the_longest_name() {
    let longest = names().longest();
    assert!(longest > "Emeric".len());
    // Every token counts as the longest name, whatever its own name's length.
    let base = "a".repeat(MAX_TEXT_LEN - longest);
    assert_eq!(
        errors(&scene(&format!("> {base}{{n:king}}\n> {base}a{{n:king}}"))),
        [format!(
            "t.dlg:5: text is {} characters; the limit is {MAX_TEXT_LEN}",
            MAX_TEXT_LEN + 1
        )]
    );
}

#[test]
fn a_rename_reaches_every_use() {
    let src = scene("> {n:king} rode out. {n:king} came back.");
    let table = from_sources(&[("t.dlg", src.as_str())], None, None, Some(&names()));
    let text = table
        .ok()
        .and_then(|t| {
            t.get("s")
                .and_then(|s| s.steps.last()?.text().map(str::to_owned))
        })
        .unwrap_or_default();
    // The stored scene keeps the tokens; the table fills them in.
    assert_eq!(text, "{n:king} rode out. {n:king} came back.");
    let mut renamed = names();
    renamed.names.insert("king".into(), "Osric".into());
    assert_eq!(
        names().substitute(&text),
        "Emeric rode out. Emeric came back."
    );
    assert_eq!(
        renamed.substitute(&text),
        "Osric rode out. Osric came back."
    );
}

/// The names example in `assets/dialogue/README.md` passes every check
/// against the real names table and reads as the README says.
#[test]
fn readme_names_example_is_valid() {
    let readme = bundle::file("dialogue/README.md").unwrap_or_default();
    let example = readme.split("```").nth(5).unwrap_or_default();
    let table = from_sources(&[("README.md", example)], None, None, Some(&names()));
    assert!(table.is_ok(), "{table:?}");
    let scene = table.unwrap_or_default().scenes.remove("ch01_road");
    let steps = scene.map(|s| s.steps).unwrap_or_default();
    let shown: Vec<String> = steps
        .iter()
        .filter_map(|s| match s {
            Step::Caption { text } => Some(text.as_str()),
            _ => s.text(),
        })
        .map(|t| names().substitute(t).into_owned())
        .collect();
    assert_eq!(
        shown,
        [
            "The Thornmarch, dusk",
            "Emeric wants you out of the Thornmarch."
        ]
    );
    assert!(readme.contains("\"The Thornmarch, dusk\""));
    assert!(readme.contains("\"Emeric wants you out of the Thornmarch.\""));
}
