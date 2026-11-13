use std::collections::HashMap;

use gengrammar::cond::{CmpOp, Cond, Operand, char_sets, eval, ranges, run_sets, set_contains};
use gengrammar::parse::parse_grammar;

const MINI: &str = include_str!("../../testdata/gengrammar/mini.c");

fn cmp(op: CmpOp, a: Operand, b: Operand) -> Cond {
    Cond::Cmp(op, a, b)
}

/// `lookahead != 0 && (lookahead < '(' || ')' < lookahead)`
fn sample() -> Cond {
    Cond::And(
        Box::new(cmp(CmpOp::Ne, Operand::Lookahead, Operand::Lit(0))),
        Box::new(Cond::Or(
            Box::new(cmp(CmpOp::Lt, Operand::Lookahead, Operand::Lit(40))),
            Box::new(cmp(CmpOp::Lt, Operand::Lit(41), Operand::Lookahead)),
        )),
    )
}

#[test]
fn mini_state0_run_sets() {
    let g = parse_grammar(MINI).unwrap();
    let sets = char_sets(&g);
    assert_eq!(sets["set_1"], [(97, 122), (0xaa, 0xaa)]);
    let case = &g.lex_fns[0].cases[0];
    assert_eq!(case.state, 0);
    let (adv, skip) = run_sets(case, &sets);
    assert!(adv.iter().all(|b| !b));
    let on: Vec<usize> = (0..128).filter(|&i| skip[i]).collect();
    assert_eq!(on, [9, 10, 11, 12, 13, 32]);
}

#[test]
fn set_1_ascii_bitmap() {
    let sets = HashMap::from([("set_1".to_string(), vec![(97, 122), (0xaa, 0xaa)])]);
    let c = Cond::SetContains {
        set: "set_1".into(),
        n: 2,
    };
    for ch in 0..128 {
        assert_eq!(eval(&c, &sets, ch, false), (97..=122).contains(&ch), "{ch}");
    }
}

#[test]
fn set_contains_matches_linear_scan() {
    let tables: [Vec<(i32, i32)>; 5] = [
        vec![],
        vec![(5, 5)],
        vec![(97, 122), (0xaa, 0xaa)],
        vec![
            (1, 3),
            (10, 20),
            (100, 200),
            (0x300, 0x400),
            (0x10000, 0x10FFFF),
        ],
        // odd length: the binary search is not a membership test, so compare to a port-free model
        vec![(1, 2), (4, 5), (7, 8)],
    ];
    for t in &tables[..4] {
        for v in -2..=0x110000i64 {
            let lin = t.iter().any(|&(s, e)| v >= s as i64 && v <= e as i64);
            assert_eq!(set_contains(t, v), lin, "{t:?} {v}");
        }
    }
    // Odd length (3 ranges): size 3 -> half 1, mid 1 = (4,5); lookahead > 5 moves index to 1,
    // size 2 -> half 1, mid 2 = (7,8); final check is range at index 1 or 2.
    let t = &tables[4];
    for v in -2..=20i64 {
        let (mut index, mut size) = (0usize, t.len());
        let mut hit = false;
        while size > 1 {
            let half = size / 2;
            let mid = index + half;
            let (s, e) = (t[mid].0 as i64, t[mid].1 as i64);
            if v >= s && v <= e {
                hit = true;
                break;
            } else if v > e {
                index = mid;
            }
            size -= half;
        }
        let want = hit || (t[index].0 as i64..=t[index].1 as i64).contains(&v);
        assert_eq!(set_contains(t, v), want, "{v}");
    }
}

#[test]
fn ranges_example() {
    let sets = HashMap::new();
    assert_eq!(
        ranges(&sample(), &sets),
        [(-1, -1), (1, 39), (42, 0x10FFFF)]
    );
    assert_eq!(ranges(&Cond::Eof, &sets), []);
}

#[test]
fn eval_points_and_eof() {
    let sets = HashMap::new();
    let c = sample();
    for v in [-1i64, 0, 40, 41, 42] {
        assert_eq!(
            eval(&c, &sets, v, false),
            v != 0 && !(40..=41).contains(&v),
            "{v}"
        );
    }
    assert!(eval(&Cond::Eof, &sets, 5, true));
    assert!(!eval(&Cond::Eof, &sets, 5, false));
    assert!(!eval(&Cond::Not(Box::new(Cond::Eof)), &sets, 5, true));
}
