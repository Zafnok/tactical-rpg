//! Chapter and New Game file tests, against the embedded content.

use trpg_core::{LeadGender, Outcome};

use super::*;

fn content() -> Content {
    crate::load_embedded().unwrap_or_else(|e| panic!("{e}"))
}

fn chapter(id: &str) -> ChapterDef {
    ChapterDef {
        id: id.into(),
        title: "T".into(),
        intro_scenes: vec!["test".into()],
        battle: "test".into(),
        victory_scenes: vec!["test_victory".into()],
        next: None,
    }
}

fn messages(errors: Vec<ContentError>) -> Vec<String> {
    errors.into_iter().map(|e| e.message).collect()
}

#[test]
fn embedded_chapters_and_new_game_load() {
    let c = content();
    let test = &c.chapters["test"];
    assert_eq!(test.battle, "test");
    assert_eq!(test.intro_scenes, ["test_intro"]);
    assert_eq!(test.victory_scenes, ["test_victory"]);
    assert_eq!(test.next, None);
    assert!(c.chapters["quick"].intro_scenes.is_empty());
    assert_eq!(c.new_game.first_chapter, "test");
    assert!(c.new_game.roster.iter().any(|r| r.0 == LEAD_ID));
}

#[test]
fn chapter_errors() {
    let c = content();
    let check =
        |def: &ChapterDef| messages(check_chapter("c.ron", "t", def, &c.battles, &c.dialogue));
    assert_eq!(check(&chapter("t")), Vec::<String>::new());
    assert_eq!(
        check(&chapter("x")),
        ["id \"x\" must match the file name \"t\""]
    );
    let bad = ChapterDef {
        battle: "nope".into(),
        intro_scenes: vec!["a".into()],
        victory_scenes: vec!["b".into()],
        ..chapter("t")
    };
    assert_eq!(
        check(&bad),
        [
            "no battle \"nope\"",
            "no dialogue scene \"a\"",
            "no dialogue scene \"b\""
        ]
    );
}

#[test]
fn next_must_be_a_chapter() {
    let mut chapters = BTreeMap::from([
        ("a".to_owned(), chapter("a")),
        ("b".to_owned(), chapter("b")),
    ]);
    assert!(check_next(&chapters).is_empty());
    chapters.get_mut("a").unwrap().next = Some("b".into());
    assert!(check_next(&chapters).is_empty());
    chapters.get_mut("b").unwrap().next = Some("z".into());
    let e = check_next(&chapters);
    assert_eq!(messages(e.clone()), ["next: no chapter \"z\""]);
    assert_eq!(e[0].file, "assets/chapters/b.ron");
}

#[test]
fn a_chapter_file_parses() {
    let def: ChapterDef = parse_ron(
        "c.ron",
        r#"(id: "t", title: "Chapter 1: Ash", battle: "b", next: Some("u"))"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(def.title, "Chapter 1: Ash");
    assert!(def.intro_scenes.is_empty() && def.victory_scenes.is_empty());
    assert_eq!(def.next.as_deref(), Some("u"));
}

#[test]
fn new_game_errors() {
    let c = content();
    let load =
        |source: &str| new_game_from_source("n.ron", source, &c.chapters, &c.characters, &c.items);
    let ok =
        r#"(first_chapter: "test", roster: ["lead"], gold: 9, stock: ["potion", "iron_sword"])"#;
    let def = load(ok).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(def.gold, 9);
    let bad =
        r#"(first_chapter: "nope", roster: ["test_knight", "x", "test_knight"], stock: ["y"])"#;
    assert_eq!(
        messages(load(bad).err().unwrap_or_default()),
        [
            "first_chapter: no chapter \"nope\"",
            "roster: no character \"x\"",
            "roster: \"test_knight\" is listed twice",
            "roster: the lead (\"lead\") must be in it",
            "stock: no item \"y\"",
        ]
    );
    assert_eq!(load("(").err().map(|e| e.len()), Some(1));
}

#[test]
fn a_new_campaign_starts_as_the_new_game_file_says() {
    let mut c = content();
    c.new_game.gold = 30;
    c.new_game.stock = vec![
        ItemId::new("potion"),
        ItemId::new("iron_sword"),
        ItemId::new("nope"),
    ];
    let lead = LeadProfile::new("Mara", LeadGender::Female);
    let game = new_campaign(&c, GameMode::Casual, lead.clone());
    assert_eq!(game.mode, GameMode::Casual);
    assert_eq!(game.lead, lead);
    assert_eq!(game.chapter, "test");
    assert_eq!(game.gold, 30);
    assert_eq!(game.stock.count(&ItemId::new("potion")), 1);
    assert_eq!(game.stock.weapons.len(), 1);
    assert_eq!(game.stock.weapons[0].def, ItemId::new("iron_sword"));
    let names: Vec<_> = game.roster.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["Mara", "Test Knight"]);
    assert!(game.roster.iter().all(|u| u.faction == Faction::Player));
    // Unknown characters are skipped.
    assert_eq!(roster_units(&c, &[CharacterId("x".into())]), []);
}

#[test]
fn a_battle_campaign_has_the_battles_characters() {
    let c = content();
    let quick = &c.battles["quick"];
    let lead = LeadProfile::new("Ellery", LeadGender::Male);
    let game = battle_campaign(&c, quick, GameMode::Classic, lead);
    let names: Vec<_> = game.roster.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        names,
        ["Test Lord", "Test Knight", "Test Archer", "Test Mage"]
    );
    assert_eq!(game.gold, 0);
    assert_eq!(game.stock, Stock::default());
}

#[test]
fn the_tables_are_the_contents() {
    let c = content();
    let t = c.tables();
    assert_eq!(*t.terrain, c.terrain.rules);
    assert_eq!(*t.classes, c.classes);
    assert_eq!(*t.items, c.items);
    assert_eq!(*t.spells, c.spells);
    assert_eq!(*t.skills, c.skills);
    assert_eq!(*t.arts, c.arts);
}

/// Every battle file plays: its setup builds with its own characters,
/// and every chapter's battle can be won or lost (it starts undecided).
#[test]
fn every_battle_starts_undecided() {
    let c = content();
    let tables = c.tables();
    for (id, def) in &c.battles {
        let lead = LeadProfile::new("Ellery", LeadGender::Male);
        let game = battle_campaign(&c, def, GameMode::Classic, lead);
        let setup = game.battle_setup(def, &tables);
        assert_eq!(
            setup.units.len(),
            def.player_slots.len() + def.enemies.len(),
            "{id}"
        );
        let (state, _) = trpg_core::BattleState::new(setup);
        assert_ne!(state.outcome(), Some(Outcome::Victory), "{id}");
        assert_ne!(state.outcome(), Some(Outcome::Defeat), "{id}");
    }
}
