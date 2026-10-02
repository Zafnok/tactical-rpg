use std::sync::LazyLock;

use super::*;

static CONTENT: LazyLock<Content> =
    LazyLock::new(|| trpg_content::load_embedded().unwrap_or_else(|e| panic!("content: {e}")));

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// A fresh, empty folder for test `name`.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-playtest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fall(unit: &str, turn: Turn) -> Fall {
    Fall {
        unit: unit.to_owned(),
        turn,
        lord: false,
    }
}

fn a_try(
    seed: u64,
    result: TryResult,
    turns: Turn,
    fallen: Vec<Fall>,
    items: &[(&str, u32)],
) -> Try {
    Try {
        seed,
        result,
        turns,
        fallen,
        items: items.iter().map(|&(i, n)| (i.to_owned(), n)).collect(),
        commands: 10,
    }
}

/// Five tries: three won, one lost with the lord fallen, one at the cap.
fn five_tries() -> Vec<Try> {
    let lord = Fall {
        unit: "Shuyi".to_owned(),
        turn: 9,
        lord: true,
    };
    vec![
        a_try(1, TryResult::Won, 14, vec![], &[("vulnerary", 2)]),
        a_try(2, TryResult::Won, 11, vec![fall("Mira", 6)], &[]),
        a_try(
            3,
            TryResult::Lost,
            9,
            vec![fall("Mira", 6), fall("Kael", 8), lord],
            &[("vulnerary", 1), ("antidote", 1)],
        ),
        a_try(4, TryResult::TurnCap, 60, vec![fall("Kael", 3)], &[]),
        a_try(
            5,
            TryResult::Won,
            22,
            vec![
                fall("Mira", 2),
                fall("Kael", 4),
                fall("Bors", 5),
                fall("Anna", 5),
            ],
            &[("vulnerary", 4)],
        ),
    ]
}

fn record(tries: Vec<Try>) -> Record {
    Record {
        date: "2026-10-01 14:05 UTC".to_owned(),
        commit: "abc1234".to_owned(),
        commands_per_second: 41_200,
        report: Report {
            battle: "ch01".to_owned(),
            mode: "Classic".to_owned(),
            bot: "baseline".to_owned(),
            seed: 1,
            runs: u32::try_from(tries.len()).unwrap(),
            turn_cap: 60,
            stats: Stats::of(&tries),
            tries,
        },
    }
}

#[test]
fn parse_args_defaults() {
    let options = parse_args(&args(&["quick"])).unwrap();
    assert_eq!(options, Options::new("quick"));
    assert_eq!(options.battle, "quick");
    assert_eq!((options.runs, options.seed, options.turn_cap), (100, 1, 60));
    assert_eq!(options.mode, GameMode::Classic);
    assert_eq!(options.bot, Bot::Baseline);
    assert_eq!(
        (options.json, options.history, options.threads),
        (None, None, None)
    );
}

#[test]
fn parse_args_reads_every_option() {
    let options = parse_args(&args(&[
        "--runs",
        "20",
        "ch01",
        "--seed",
        "7",
        "--mode",
        "casual",
        "--turn-cap",
        "30",
        "--bot",
        "baseline",
        "--json",
        "out.json",
        "--history",
        "runs",
        "--threads",
        "2",
    ]))
    .unwrap();
    let expected = Options {
        battle: "ch01".to_owned(),
        runs: 20,
        seed: 7,
        mode: GameMode::Casual,
        turn_cap: 30,
        bot: Bot::Baseline,
        json: Some(PathBuf::from("out.json")),
        history: Some(PathBuf::from("runs")),
        threads: Some(2),
    };
    assert_eq!(options, expected);
    let classic = parse_args(&args(&["ch01", "--mode", "classic", "--seed", "0"])).unwrap();
    assert_eq!((classic.mode, classic.seed), (GameMode::Classic, 0));
}

#[test]
fn parse_args_refuses_bad_arguments() {
    let error = |items: &[&str]| parse_args(&args(items)).unwrap_err();
    assert_eq!(error(&[]), "no battle given");
    assert_eq!(error(&["--runs", "5"]), "no battle given");
    assert_eq!(error(&["a", "b"]), "unexpected argument: b");
    assert_eq!(error(&["a", "--bogus", "1"]), "unknown option: --bogus");
    assert_eq!(error(&["a", "--runs"]), "--runs requires a value");
    assert_eq!(error(&["a", "--runs", "0"]), "--runs: bad value \"0\"");
    assert_eq!(error(&["a", "--runs", "x"]), "--runs: bad value \"x\"");
    assert_eq!(error(&["a", "--seed", "-1"]), "--seed: bad value \"-1\"");
    assert_eq!(
        error(&["a", "--mode", "hard"]),
        "--mode: bad value \"hard\""
    );
    assert_eq!(
        error(&["a", "--turn-cap", "0"]),
        "--turn-cap: bad value \"0\""
    );
    assert_eq!(
        error(&["a", "--bot", "casual"]),
        "--bot: bad value \"casual\""
    );
    assert_eq!(
        error(&["a", "--threads", "0"]),
        "--threads: bad value \"0\""
    );
}

#[test]
fn stats_count_the_tries() {
    let stats = Stats::of(&five_tries());
    let expected = Stats {
        won: 3,
        lost: 1,
        turn_cap: 1,
        win_percent: 60,
        turns_median: 14,
        turns_min: 9,
        turns_max: 60,
        fallen_total: 9,
        fallen_max: 4,
        fallen_per_try: [1, 2, 0, 2],
        fell_most: vec![
            ("Kael".to_owned(), 3),
            ("Mira".to_owned(), 3),
            ("Anna".to_owned(), 1),
            ("Bors".to_owned(), 1),
            ("Shuyi".to_owned(), 1),
        ],
        items_total: 8,
        items: [("antidote".to_owned(), 1), ("vulnerary".to_owned(), 7)].into(),
    };
    assert_eq!(stats, expected);
}

#[test]
fn stats_of_no_tries_are_zero() {
    assert_eq!(Stats::of(&[]), Stats::default());
}

#[test]
fn stats_median_is_the_lower_middle_try() {
    let turns = |turns: &[Turn]| {
        let tries: Vec<Try> = turns
            .iter()
            .map(|&t| a_try(1, TryResult::Won, t, vec![], &[]))
            .collect();
        Stats::of(&tries).turns_median
    };
    assert_eq!(turns(&[4]), 4);
    assert_eq!(turns(&[9, 3]), 3);
    assert_eq!(turns(&[9, 3, 5]), 5);
    assert_eq!(turns(&[9, 3, 7, 5]), 5);
    assert_eq!(turns(&[9, 3, 7, 5, 11]), 7);
}

#[test]
fn stats_win_percent_is_rounded() {
    let percent = |won: usize, lost: usize| {
        let mut tries = vec![a_try(1, TryResult::Won, 1, vec![], &[]); won];
        tries.extend(vec![a_try(1, TryResult::Lost, 1, vec![], &[]); lost]);
        Stats::of(&tries).win_percent
    };
    assert_eq!(percent(1, 2), 33);
    assert_eq!(percent(2, 1), 67);
    assert_eq!(percent(1, 7), 13);
    assert_eq!(percent(3, 0), 100);
    assert_eq!(percent(0, 3), 0);
}

#[test]
fn stats_two_fallen_go_in_their_own_column() {
    let tries = [a_try(
        1,
        TryResult::Won,
        1,
        vec![fall("Mira", 1), fall("Kael", 1)],
        &[],
    )];
    assert_eq!(Stats::of(&tries).fallen_per_try, [0, 0, 1, 0]);
}

#[test]
fn render_matches_the_layout() {
    let text = render(&record(five_tries()), None);
    let expected = "\
ch01 · Classic · baseline · 5 tries (seeds 1–5)
won 3  lost 1  turn cap 1   (60%)
turns     median 14  min 9  max 60
fallen    mean 1.8   max 4
items     mean 1.6   (vulnerary 1.4, antidote 0.2)
fallen/try 0: 1  1: 2  2: 0  3+: 2   most: Kael 3×, Mira 3×, Anna 1×
speed     41 200 commands/s

try 1 · won · fell: nobody · items 2 · turns 14
try 2 · won · fell: Mira T6 · items 0 · turns 11
try 3 · lost (lord fell T9) · fell: Mira T6, Kael T8, Shuyi T9 · items 2 · turns 9
try 4 · turn cap · fell: Kael T3 · items 0 · turns 60
try 5 · won · fell: Mira T2, Kael T4, Bors T5, Anna T5 · items 4 · turns 22
";
    assert_eq!(text, expected);
}

#[test]
fn render_shows_the_previous_runs_numbers() {
    let mut previous = record(vec![
        a_try(
            11,
            TryResult::Lost,
            8,
            vec![fall("Mira", 6)],
            &[("vulnerary", 1)],
        ),
        a_try(12, TryResult::Won, 12, vec![], &[]),
        a_try(13, TryResult::Lost, 30, vec![], &[]),
        a_try(14, TryResult::Lost, 4, vec![], &[("vulnerary", 2)]),
    ]);
    previous.date = "2026-09-30 08:00 UTC".to_owned();
    previous.commit = "fd6920d".to_owned();
    previous.report.seed = 11;
    let text = render(&record(five_tries()), Some(&previous));
    let head: Vec<&str> = text.lines().take(9).collect();
    let expected = [
        "ch01 · Classic · baseline · 5 tries (seeds 1–5)",
        "won 3  lost 1  turn cap 1   (60%, was 25%)",
        "turns     median 14 (was 8)  min 9  max 60",
        "fallen    mean 1.8 (was 0.2)   max 4",
        "items     mean 1.6 (was 0.8)   (vulnerary 1.4, antidote 0.2)",
        "fallen/try 0: 1  1: 2  2: 0  3+: 2   most: Kael 3×, Mira 3×, Anna 1×",
        "speed     41 200 commands/s",
        "previous  2026-09-30 08:00 UTC · commit fd6920d · 4 tries (seeds 11–14)",
        "",
    ];
    assert_eq!(head, expected);
    assert_eq!(text.lines().count(), 9 + 5);
}

#[test]
fn render_with_nothing_fallen_or_used() {
    let text = render(
        &record(vec![a_try(1, TryResult::Won, 3, vec![], &[])]),
        None,
    );
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[3], "fallen    mean 0.0   max 0");
    assert_eq!(lines[4], "items     mean 0.0");
    assert_eq!(
        lines[5],
        "fallen/try 0: 1  1: 0  2: 0  3+: 0   most: nobody"
    );
}

#[test]
fn try_line_says_how_a_loss_came() {
    let lost = |fallen| try_line(&a_try(17, TryResult::Lost, 9, fallen, &[("vulnerary", 2)]));
    let lord = Fall {
        unit: "Kael".to_owned(),
        turn: 9,
        lord: true,
    };
    assert_eq!(
        lost(vec![fall("Mira", 6), lord]),
        "try 17 · lost (lord fell T9) · fell: Mira T6, Kael T9 · items 2 · turns 9"
    );
    assert_eq!(
        lost(vec![fall("Mira", 6)]),
        "try 17 · lost · fell: Mira T6 · items 2 · turns 9"
    );
    assert_eq!(
        lost(vec![]),
        "try 17 · lost · fell: nobody · items 2 · turns 9"
    );
}

#[test]
fn mean_has_one_decimal() {
    assert_eq!(mean(9, 5), "1.8");
    assert_eq!(mean(0, 3), "0.0");
    assert_eq!(mean(31, 10), "3.1");
    assert_eq!(mean(5, 0), "5.0");
}

#[test]
fn grouped_puts_a_space_every_three_digits() {
    assert_eq!(grouped(0), "0");
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(1000), "1 000");
    assert_eq!(grouped(41_200), "41 200");
    assert_eq!(grouped(1_234_567), "1 234 567");
}

#[test]
fn per_second_divides_by_the_time_played() {
    assert_eq!(per_second(300, Duration::from_secs(2)), 150);
    assert_eq!(per_second(7, Duration::from_millis(500)), 14);
    assert_eq!(per_second(1, Duration::from_secs(3)), 0);
    assert_eq!(per_second(5, Duration::ZERO), 0);
}

#[test]
fn utc_stamp_gives_the_date_and_time() {
    let day = 86_400;
    assert_eq!(utc_stamp(0), "1970-01-01 00:00 UTC");
    assert_eq!(utc_stamp(59), "1970-01-01 00:00 UTC");
    assert_eq!(utc_stamp(60), "1970-01-01 00:01 UTC");
    assert_eq!(utc_stamp(day - 1), "1970-01-01 23:59 UTC");
    assert_eq!(utc_stamp(day), "1970-01-02 00:00 UTC");
    assert_eq!(utc_stamp(30 * day), "1970-01-31 00:00 UTC");
    assert_eq!(utc_stamp(31 * day), "1970-02-01 00:00 UTC");
    assert_eq!(utc_stamp(364 * day), "1970-12-31 00:00 UTC");
    assert_eq!(utc_stamp(365 * day), "1971-01-01 00:00 UTC");
    // 1972 is a leap year.
    assert_eq!(utc_stamp(789 * day), "1972-02-29 00:00 UTC");
    assert_eq!(utc_stamp(790 * day), "1972-03-01 00:00 UTC");
    assert_eq!(utc_stamp(1095 * day), "1972-12-31 00:00 UTC");
    assert_eq!(utc_stamp(1096 * day), "1973-01-01 00:00 UTC");
    // 2000 is one (divisible by 400); 2100 is not (by 100).
    assert_eq!(utc_stamp(951_782_400), "2000-02-29 00:00 UTC");
    assert_eq!(utc_stamp(4_107_456_000), "2100-02-28 00:00 UTC");
    assert_eq!(utc_stamp(4_107_542_400), "2100-03-01 00:00 UTC");
    assert_eq!(utc_stamp(1_790_863_500), "2026-10-01 14:05 UTC");
}

#[test]
fn now_is_after_2023() {
    assert!(now() > 1_700_000_000);
}

#[test]
fn git_commit_is_the_short_hash_or_unknown() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let commit = git_commit(&root);
    // A copy of the sources without their history (cargo-mutants makes
    // one) has no commit.
    if root.join(".git").exists() {
        assert!(commit.len() >= 7, "{commit}");
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()), "{commit}");
    } else {
        assert_eq!(commit, "unknown");
    }
    let nowhere = temp_dir("no-repo");
    assert_eq!(git_commit(&nowhere), "unknown");
    std::fs::remove_dir_all(&nowhere).unwrap();
}

#[test]
fn history_keeps_every_run_and_gives_the_last() {
    let dir = temp_dir("history");
    let first = record(five_tries());
    let file = history_file(&dir.join("kept"), &first.report);
    assert_eq!(file, dir.join("kept").join("ch01-classic-baseline.jsonl"));
    assert_eq!(last_record(&file), None);

    append(&file, &first).unwrap();
    assert_eq!(last_record(&file), Some(first.clone()));
    let mut second = record(vec![a_try(1, TryResult::Won, 3, vec![], &[])]);
    second.commit = "second".to_owned();
    append(&file, &second).unwrap();
    assert_eq!(last_record(&file), Some(second));
    assert_eq!(std::fs::read_to_string(&file).unwrap().lines().count(), 2);

    // A line that isn't a run (an older format, a cut-off write) is passed
    // over.
    let mut text = std::fs::read_to_string(&file).unwrap();
    text.push_str("{\"date\": \n");
    std::fs::write(&file, text).unwrap();
    assert_eq!(
        last_record(&file).map(|r| r.commit),
        Some("second".to_owned())
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn history_has_a_file_per_battle_mode_and_bot() {
    let mut report = record(vec![]).report;
    report.battle = "quick".to_owned();
    report.mode = "Casual".to_owned();
    let file = history_file(Path::new("h"), &report);
    assert_eq!(file, Path::new("h").join("quick-casual-baseline.jsonl"));
}

/// The Quick Battle, before its first command.
fn quick_state() -> BattleState {
    let def = &CONTENT.battles["quick"];
    let lead = LeadProfile::new(LEAD_NAME, LeadGender::Male);
    let campaign = battle_campaign(&CONTENT, def, GameMode::Classic, lead);
    BattleState::new(campaign.battle_setup(def, &CONTENT.tables())).0
}

#[test]
fn cast_names_every_unit_and_the_lord() {
    let state = quick_state();
    let cast = cast(&state);
    assert!(!state.reinforcements().is_empty());
    assert_eq!(
        cast.len(),
        state.units().len() + state.reinforcements().len()
    );
    for unit in state.units() {
        assert_eq!(cast[&unit.id], (unit.name.clone(), unit.is_lord));
    }
    assert_eq!(cast.values().filter(|(_, lord)| *lord).count(), 1);
}

#[test]
fn to_try_reads_the_measures() {
    let state = quick_state();
    let cast = cast(&state);
    let lord = state.units().iter().find(|u| u.is_lord).unwrap();
    let other = state.units().iter().find(|u| !u.is_lord).unwrap();

    let mut measures = BattleMeasures::new(&state);
    measures.outcome = Some(Outcome::Defeat);
    measures.turns = 9;
    measures.commands = 123;
    measures.player_fallen = vec![(other.id, 6), (UnitId(999), 7), (lord.id, 9)];
    measures.items_used = [(trpg_core::ItemId::new("potion"), 2)].into();
    let lost = to_try(17, &measures, &cast, 60);
    let fell = |unit: &str, turn, lord| Fall {
        unit: unit.to_owned(),
        turn,
        lord,
    };
    let expected = Try {
        seed: 17,
        result: TryResult::Lost,
        turns: 9,
        fallen: vec![
            fell(&other.name, 6, false),
            fell("unit 999", 7, false),
            fell(&lord.name, 9, true),
        ],
        items: [("potion".to_owned(), 2)].into(),
        commands: 123,
    };
    assert_eq!(lost, expected);

    measures.outcome = Some(Outcome::Victory);
    assert_eq!(to_try(1, &measures, &cast, 60).result, TryResult::Won);
    // A try stopped by the cap saw the next turn start; it took the cap.
    measures.outcome = None;
    measures.turns = 61;
    let capped = to_try(1, &measures, &cast, 60);
    assert_eq!((capped.result, capped.turns), (TryResult::TurnCap, 60));
}

fn quick(runs: u32) -> Options {
    Options {
        runs,
        ..Options::new("quick")
    }
}

#[test]
fn play_reports_every_try_by_seed() {
    let options = Options {
        seed: 5,
        mode: GameMode::Casual,
        turn_cap: 40,
        ..quick(6)
    };
    let (report, speed) = play(&CONTENT, &options).unwrap();
    assert_eq!(report.battle, "quick");
    assert_eq!(report.mode, "Casual");
    assert_eq!(report.bot, "baseline");
    assert_eq!((report.seed, report.runs, report.turn_cap), (5, 6, 40));
    let seeds: Vec<u64> = report.tries.iter().map(|t| t.seed).collect();
    assert_eq!(seeds, [5, 6, 7, 8, 9, 10]);
    assert_eq!(report.stats, Stats::of(&report.tries));
    assert!(report.tries.iter().all(|t| t.commands > 0));
    assert!(speed > 0);
}

#[test]
fn play_gives_the_same_report_for_the_same_arguments() {
    let (first, _) = play(&CONTENT, &quick(20)).unwrap();
    let (second, _) = play(&CONTENT, &quick(20)).unwrap();
    assert_eq!(first, second);
    // Luck is fresh each try: the Quick Battle goes both ways.
    assert!(
        first.stats.won > 0 && first.stats.lost > 0,
        "{:?}",
        first.stats
    );
}

#[test]
fn play_gives_the_same_report_on_any_thread_count() {
    let on = |threads| Options {
        threads: Some(threads),
        ..quick(20)
    };
    let (one, _) = play(&CONTENT, &on(1)).unwrap();
    assert_eq!(one.tries.len(), 20);
    for threads in [2, 3, 32] {
        let (many, _) = play(&CONTENT, &on(threads)).unwrap();
        assert_eq!(many, one, "{threads} threads");
    }
    let (every_core, _) = play(&CONTENT, &quick(20)).unwrap();
    assert_eq!(every_core, one);
}

#[test]
fn a_try_depends_only_on_its_seed() {
    let (batch, _) = play(&CONTENT, &quick(6)).unwrap();
    let alone = Options {
        seed: 5,
        ..quick(1)
    };
    let (alone, _) = play(&CONTENT, &alone).unwrap();
    assert_eq!(alone.tries, [batch.tries[4].clone()]);
}

#[test]
fn play_refuses_an_unknown_battle() {
    let error = play(&CONTENT, &Options::new("nope")).unwrap_err();
    assert!(
        error.starts_with("no battle \"nope\" in assets/battles (there: "),
        "{error}"
    );
    assert!(error.contains("quick"), "{error}");
}

#[test]
fn run_keeps_the_run_and_compares_the_next_with_it() {
    let dir = temp_dir("run");
    let json = dir.join("out").with_extension("json");
    let options = Options {
        json: Some(json.clone()),
        history: Some(dir.join("history")),
        ..quick(4)
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let first = run(&root, &options).unwrap();
    let written = std::fs::read_to_string(&json).unwrap();
    assert!(written.ends_with("}\n"));
    let record: Record = serde_json::from_str(&written).unwrap();
    assert_eq!(first, render(&record, None));
    assert_eq!(record.report, play(&CONTENT, &options).unwrap().0);
    assert_eq!(record.commit, git_commit(&root));
    assert!(record.date.starts_with("20") && record.date.ends_with(" UTC"));
    assert_eq!(record.date.len(), "2026-10-01 14:05 UTC".len());
    let kept = dir.join("history").join("quick-classic-baseline.jsonl");
    assert_eq!(last_record(&kept), Some(record.clone()));

    let second = run(&root, &options).unwrap();
    let again: Record = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    assert_eq!(second, render(&again, Some(&record)));
    assert_eq!(again.report, record.report);
    assert!(second.contains("%, was "), "{second}");
    assert!(second.contains("\nprevious  20"), "{second}");
    assert_eq!(std::fs::read_to_string(&kept).unwrap().lines().count(), 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn run_keeps_history_under_target_by_default() {
    // A stand-in repo root, so the real history isn't touched.
    let root = temp_dir("default-history");
    run(&root, &quick(2)).unwrap();
    let kept = root.join("target/playtest-history/quick-classic-baseline.jsonl");
    let record = last_record(&kept).unwrap();
    assert_eq!(record.report.runs, 2);
    assert_eq!(record.commit, "unknown");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn run_fails_on_an_unknown_battle() {
    let root = temp_dir("unknown-battle");
    assert!(run(&root, &Options::new("nope")).is_err());
    assert!(!root.join("target").exists());
    std::fs::remove_dir_all(&root).unwrap();
}
