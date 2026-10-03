//! Tests of the support rules, with the design's starting numbers
//! (`supports.md`): C at 20 points, B at 80, A at 180.

use proptest::prelude::*;

use super::*;

const RULES: SupportRules = SupportRules::STARTING;
const T: Thresholds = RULES.thresholds;

fn c(id: &str) -> CharacterId {
    CharacterId(id.into())
}

fn pair(a: &str, b: &str) -> SupportPair {
    SupportPair::new(c(a), c(b))
}

fn def(a: &str, b: &str) -> PairDef {
    PairDef {
        pair: pair(a, b),
        thresholds: None,
        starting_points: 0,
        conversations: ByRank {
            c: format!("{a}_{b}_c"),
            b: format!("{a}_{b}_b"),
            a: format!("{a}_{b}_a"),
        },
    }
}

/// `ann`–`ben` with the rules' thresholds; `ann`–`cal` lifelong friends
/// (C at 0); `ben`–`cal` a slow burn starting with 30 points.
fn table() -> SupportTable {
    let friends = PairDef {
        thresholds: Some(ByRank {
            c: 0,
            b: 50,
            a: 100,
        }),
        ..def("ann", "cal")
    };
    let slow = PairDef {
        thresholds: Some(ByRank {
            c: 40,
            b: 120,
            a: 300,
        }),
        starting_points: 30,
        ..def("ben", "cal")
    };
    SupportTable::new(RULES, [def("ann", "ben"), friends, slow])
}

/// What a state holds: points, rank, unlocked rank, ended.
fn parts(s: SupportState) -> (u32, Option<SupportRank>, Option<SupportRank>, bool) {
    (s.points(), s.rank(), s.unlocked(), s.ended())
}

use SupportRank::{A, B, C};

// ---- Ranks and pairs -----------------------------------------------------------

#[test]
fn ranks_go_c_b_a() {
    assert_eq!(SupportRank::ALL, [C, B, A]);
    assert!(C < B && B < A);
    assert_eq!(SupportRank::after(None), Some(C));
    assert_eq!(SupportRank::after(Some(C)), Some(B));
    assert_eq!(SupportRank::after(Some(B)), Some(A));
    assert_eq!(SupportRank::after(Some(A)), None);
    let by = ByRank { c: 1, b: 2, a: 3 };
    assert_eq!(SupportRank::ALL.map(|r| *by.get(r)), [1, 2, 3]);
}

#[test]
fn the_starting_rules_are_the_designs() {
    assert_eq!(
        T,
        ByRank {
            c: 20,
            b: 80,
            a: 180
        }
    );
    let p = RULES.points;
    assert_eq!(
        [
            p.adjacent_at_phase_end,
            p.fight_beside,
            p.heal,
            p.buff,
            p.item
        ],
        [1, 3, 3, 3, 2]
    );
    assert_eq!(RULES.bonus_range, 3);
    assert_eq!(
        SupportRank::ALL.map(|r| *RULES.bonus.get(r)),
        [5, 10, 15].map(|n| SupportBonus { hit: n, avoid: n })
    );
    // The default is no supports at all.
    assert_eq!(SupportRules::default().bonus_range, 0);
    assert_eq!(SupportTable::default().pairs().count(), 0);
}

#[test]
fn thresholds_must_rise_with_each_rank() {
    assert!(T.is_increasing());
    for (c, b, a) in [(20, 20, 180), (20, 80, 80), (80, 20, 180), (20, 180, 80)] {
        assert!(!ByRank { c, b, a }.is_increasing(), "{c} {b} {a}");
    }
}

#[test]
fn a_pair_is_the_same_either_way_round() {
    assert_eq!(pair("ann", "ben"), pair("ben", "ann"));
    assert_eq!(pair("ben", "ann").members(), [&c("ann"), &c("ben")]);
    assert_ne!(pair("ann", "ben"), pair("ann", "cal"));
    let p = pair("ben", "ann");
    assert!(p.has(&c("ann")) && p.has(&c("ben")));
    assert!(!p.has(&c("cal")));
    // A saved pair comes back in order whatever order it was written in.
    let loaded: SupportPair = ron::from_str("(\"ben\", \"ann\")").unwrap();
    assert_eq!(loaded, p);
    let text = ron::to_string(&p).unwrap();
    assert_eq!(ron::from_str::<SupportPair>(&text).unwrap(), p);
}

proptest! {
    #[test]
    fn pairs_are_symmetric(a in "[a-d]{1,2}", b in "[a-d]{1,2}", n in 0u32..30) {
        prop_assert_eq!(pair(&a, &b), pair(&b, &a));
        // (Nobody has a support with themselves.)
        prop_assume!(a != b);
        // The table and the book don't care about the order either.
        let table = SupportTable::new(RULES, [def(&a, &b)]);
        prop_assert!(table.get(&pair(&b, &a)).is_some());
        let mut book = SupportBook::default();
        let gained = book.gain(&pair(&a, &b), n, &table);
        prop_assert_eq!(gained, n.min(T.c));
        prop_assert_eq!(
            book.state(&pair(&b, &a), &table).map(|s| s.points()),
            Some(gained)
        );
    }
}

// ---- Battle bonus ------------------------------------------------------------------

#[test]
fn the_bonus_is_the_best_partners_alone() {
    let bonus = |ranks: &[SupportRank]| {
        let b = RULES.best_bonus(ranks.iter().copied());
        (b.hit, b.avoid)
    };
    assert_eq!(bonus(&[]), (0, 0));
    assert_eq!(bonus(&[C]), (5, 5));
    assert_eq!(bonus(&[B]), (10, 10));
    assert_eq!(bonus(&[A]), (15, 15));
    // Never combined: an A and a C partner give the A bonus.
    assert_eq!(bonus(&[C, A]), (15, 15));
    assert_eq!(bonus(&[A, C, B, A]), (15, 15));
    assert_eq!(bonus(&[C, B, C]), (10, 10));
    // Hit and avoid are separate numbers.
    let rules = SupportRules {
        bonus: ByRank {
            c: SupportBonus { hit: 1, avoid: 2 },
            ..RULES.bonus
        },
        ..RULES
    };
    assert_eq!(rules.best_bonus([C]), SupportBonus { hit: 1, avoid: 2 });
}

fn any_rank() -> impl Strategy<Value = SupportRank> {
    prop::sample::select(SupportRank::ALL.to_vec())
}

proptest! {
    #[test]
    fn the_bonus_never_exceeds_the_a_rank_bonus(
        ranks in prop::collection::vec(any_rank(), 0..12),
    ) {
        let bonus = RULES.best_bonus(ranks.iter().copied());
        prop_assert!(bonus.hit <= RULES.bonus.a.hit);
        prop_assert!(bonus.avoid <= RULES.bonus.a.avoid);
        let best = ranks.iter().max().map(|r| *RULES.bonus.get(*r));
        prop_assert_eq!(bonus, best.unwrap_or_default());
    }
}

// ---- Points, thresholds and viewing -------------------------------------------------

#[test]
fn points_add_up_to_a_threshold_which_unlocks_the_conversation() {
    let mut s = SupportState::start(0, &T);
    assert_eq!(parts(s), (0, None, None, false));
    assert_eq!(s.gain(3, &T), 3);
    assert_eq!(s.gain(16, &T), 16);
    assert_eq!(parts(s), (19, None, None, false));
    // The 20th point unlocks C; the rank isn't gained yet.
    assert_eq!(s.gain(1, &T), 1);
    assert_eq!(parts(s), (20, None, Some(C), false));
    assert_eq!(s.seen().count(), 0);
}

#[test]
fn points_stop_at_an_unviewed_threshold() {
    let mut s = SupportState::start(18, &T);
    // Extra points are lost.
    assert_eq!(s.gain(5, &T), 2);
    assert_eq!(parts(s), (20, None, Some(C), false));
    assert_eq!(s.gain(100, &T), 0);
    assert_eq!(parts(s), (20, None, Some(C), false));
    // Viewing gains the rank, and points count again.
    assert_eq!(s.view(), Some(C));
    assert_eq!(parts(s), (20, Some(C), None, false));
    assert_eq!(s.view(), None);
    assert_eq!(s.gain(100, &T), 60);
    assert_eq!(parts(s), (80, Some(C), Some(B), false));
    assert_eq!(s.gain(1, &T), 0);
    assert_eq!(s.view(), Some(B));
    assert_eq!(s.seen().collect::<Vec<_>>(), [C, B]);
    assert_eq!(s.gain(u32::MAX, &T), 100);
    assert_eq!(parts(s), (180, Some(B), Some(A), false));
    assert_eq!(s.view(), Some(A));
    // At A nothing more is gained.
    assert_eq!(s.gain(5, &T), 0);
    assert_eq!(parts(s), (180, Some(A), None, false));
    assert_eq!(s.seen().collect::<Vec<_>>(), [C, B, A]);
}

#[test]
fn starting_points_count_like_earned_ones() {
    assert_eq!(parts(SupportState::start(7, &T)), (7, None, None, false));
    // Starting past C: C is unlocked, and the points wait at its threshold.
    assert_eq!(
        parts(SupportState::start(500, &T)),
        (20, None, Some(C), false)
    );
    // Lifelong friends: C needs no points.
    let friends = ByRank {
        c: 0,
        b: 50,
        a: 100,
    };
    assert_eq!(
        parts(SupportState::start(0, &friends)),
        (0, None, Some(C), false)
    );
}

#[test]
fn a_threshold_lowered_by_a_data_change_takes_no_points() {
    let mut s = SupportState::start(15, &T);
    let lower = ByRank {
        c: 10,
        b: 80,
        a: 180,
    };
    assert_eq!(s.gain(1, &lower), 0);
    assert_eq!(parts(s), (15, None, Some(C), false));
}

#[test]
fn an_ended_support_gains_nothing_and_loses_its_unviewed_conversation() {
    let mut s = SupportState::start(20, &T);
    s.view();
    s.gain(60, &T);
    assert_eq!(parts(s), (80, Some(C), Some(B), false));
    s.end();
    // The viewed rank stays (the "seen" list).
    assert_eq!(parts(s), (80, Some(C), None, true));
    assert_eq!(s.seen().collect::<Vec<_>>(), [C]);
    assert_eq!(s.view(), None);
    let mut early = SupportState::start(3, &T);
    early.end();
    assert_eq!(early.gain(5, &T), 0);
    assert_eq!(parts(early), (3, None, None, true));
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Gain(u32),
    View,
}

fn any_op() -> impl Strategy<Value = Op> {
    prop_oneof![(0u32..60).prop_map(Op::Gain), Just(Op::View)]
}

proptest! {
    #[test]
    fn points_never_pass_the_next_unviewed_threshold(
        c in 0u32..30,
        b in 1u32..60,
        a in 1u32..120,
        start in 0u32..200,
        ops in prop::collection::vec(any_op(), 0..40),
    ) {
        let t = ByRank { c, b: c + b, a: c + b + a };
        prop_assert!(t.is_increasing());
        let mut s = SupportState::start(start, &t);
        let mut views = 0;
        for op in std::iter::once(Op::Gain(0)).chain(ops) {
            let before = s;
            match op {
                Op::Gain(n) => {
                    let gained = s.gain(n, &t);
                    prop_assert!(gained <= n);
                    prop_assert_eq!(s.points(), before.points() + gained);
                    prop_assert_eq!(s.rank(), before.rank());
                }
                Op::View => {
                    let viewed = s.view();
                    prop_assert_eq!(viewed, before.unlocked());
                    views += usize::from(viewed.is_some());
                    prop_assert_eq!(s.points(), before.points());
                }
            }
            // Never past the next rank's threshold, or A's at rank A.
            let cap = *t.get(SupportRank::after(s.rank()).unwrap_or(A));
            prop_assert!(s.points() <= cap, "{s:?}");
            // A waiting conversation is the next rank's, at its threshold.
            if let Some(rank) = s.unlocked() {
                prop_assert_eq!(Some(rank), SupportRank::after(s.rank()));
                prop_assert_eq!(s.points(), *t.get(rank));
            }
            // One rank per viewing, however many points came in between.
            prop_assert_eq!(s.seen().count(), views);
        }
    }
}

// ---- The book -------------------------------------------------------------------

#[test]
fn only_listed_pairs_have_a_support() {
    let table = table();
    let mut book = SupportBook::default();
    let strangers = pair("ann", "dan");
    assert_eq!(table.get(&strangers), None);
    assert_eq!(book.state(&strangers, &table), None);
    assert_eq!(book.gain(&strangers, 5, &table), 0);
    assert_eq!(book.view(&strangers, &table), Err(SupportError::NoSuchPair));
    assert_eq!(book, SupportBook::default());
    assert_eq!(
        table.pairs().map(|d| &d.pair).collect::<Vec<_>>(),
        [
            &pair("ann", "ben"),
            &pair("ann", "cal"),
            &pair("ben", "cal")
        ]
    );
}

#[test]
fn nobody_has_a_support_with_themselves() {
    let table = SupportTable::new(RULES, [def("ann", "ann"), def("ann", "ben")]);
    assert_eq!(table.get(&pair("ann", "ann")), None);
    assert_eq!(table.pairs().count(), 1);
    // A pair listed twice keeps its last entry.
    let later = PairDef {
        starting_points: 9,
        ..def("ben", "ann")
    };
    let table = SupportTable::new(RULES, [def("ann", "ben"), later.clone()]);
    assert_eq!(table.pairs().collect::<Vec<_>>(), [&later]);
}

#[test]
fn a_pair_starts_from_its_data_and_uses_its_own_thresholds() {
    let table = table();
    let mut book = SupportBook::default();
    let state = |book: &SupportBook, a, b| book.state(&pair(a, b), &table).map(parts);
    assert_eq!(state(&book, "ann", "ben"), Some((0, None, None, false)));
    assert_eq!(state(&book, "ann", "cal"), Some((0, None, Some(C), false)));
    assert_eq!(state(&book, "ben", "cal"), Some((30, None, None, false)));
    // Nobody has a rank before a conversation was viewed.
    assert_eq!(book.rank(&pair("ann", "cal")), None);
    assert_eq!(book.unlocked(&table), [(pair("ann", "cal"), C)]);
    // The slow burn needs 40 for C, not 20.
    assert_eq!(book.gain(&pair("cal", "ben"), 25, &table), 10);
    assert_eq!(state(&book, "ben", "cal"), Some((40, None, Some(C), false)));
    assert_eq!(book.gain(&pair("ann", "ben"), 25, &table), 20);
    assert_eq!(
        book.unlocked(&table),
        [
            (pair("ann", "ben"), C),
            (pair("ann", "cal"), C),
            (pair("ben", "cal"), C)
        ]
    );
}

#[test]
fn viewing_a_conversation_raises_the_rank_and_names_its_scene() {
    let table = table();
    let mut book = SupportBook::default();
    let p = pair("ben", "ann");
    assert_eq!(book.view(&p, &table), Err(SupportError::NothingUnlocked));
    book.gain(&p, 20, &table);
    assert_eq!(
        book.view(&p, &table),
        Ok(SupportViewed {
            rank: C,
            conversation: "ann_ben_c".into()
        })
    );
    assert_eq!(book.rank(&p), Some(C));
    assert_eq!(book.view(&p, &table), Err(SupportError::NothingUnlocked));
    book.gain(&p, 500, &table);
    let viewed = book.view(&p, &table).unwrap();
    assert_eq!(
        (viewed.rank, viewed.conversation.as_str()),
        (B, "ann_ben_b")
    );
    book.gain(&p, 500, &table);
    let viewed = book.view(&p, &table).unwrap();
    assert_eq!(
        (viewed.rank, viewed.conversation.as_str()),
        (A, "ann_ben_a")
    );
    assert_eq!(book.rank(&p), Some(A));
    // A pair that starts unlocked can be viewed at once.
    assert_eq!(
        book.view(&pair("ann", "cal"), &table).map(|v| v.rank),
        Ok(C)
    );
    assert_eq!(book.unlocked(&table), []);
}

#[test]
fn a_dead_characters_supports_end() {
    let table = table();
    let mut book = SupportBook::default();
    book.gain(&pair("ann", "ben"), 20, &table);
    book.view(&pair("ann", "ben"), &table).unwrap();
    book.gain(&pair("ann", "ben"), 60, &table);
    book.end_for(&c("ann"), &table);
    let state = |book: &SupportBook, a, b| book.state(&pair(a, b), &table).map(parts);
    // Viewed ranks stay; unviewed conversations go, even one a pair
    // started with and never touched.
    assert_eq!(state(&book, "ann", "ben"), Some((80, Some(C), None, true)));
    assert_eq!(state(&book, "ann", "cal"), Some((0, None, None, true)));
    assert_eq!(state(&book, "ben", "cal"), Some((30, None, None, false)));
    assert_eq!(book.unlocked(&table), []);
    assert_eq!(book.gain(&pair("ann", "ben"), 5, &table), 0);
    assert_eq!(
        book.view(&pair("ann", "cal"), &table),
        Err(SupportError::NothingUnlocked)
    );
    // The living pair carries on.
    assert_eq!(book.gain(&pair("ben", "cal"), 5, &table), 5);
    assert_eq!(book.rank(&pair("ann", "ben")), Some(C));
}

#[test]
fn the_book_round_trips_through_ron() {
    let table = table();
    let mut book = SupportBook::default();
    book.gain(&pair("ann", "ben"), 20, &table);
    book.view(&pair("ann", "ben"), &table).unwrap();
    book.gain(&pair("ben", "cal"), 3, &table);
    book.end_for(&c("cal"), &table);
    let text = ron::to_string(&book).unwrap();
    assert_eq!(ron::from_str::<SupportBook>(&text).unwrap(), book);
}

#[test]
fn errors_say_what_is_wrong() {
    assert_eq!(
        SupportError::NoSuchPair.to_string(),
        "these two have no support"
    );
    assert_eq!(
        SupportError::NotInArmy(c("ann")).to_string(),
        "ann isn't in the army"
    );
    assert_eq!(
        SupportError::NothingUnlocked.to_string(),
        "no support conversation is unlocked"
    );
}
