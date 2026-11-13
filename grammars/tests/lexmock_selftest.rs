//! The mock's digest depends on exactly the calls a lex function makes.

#[path = "common/lexmock.rs"]
mod lexmock;

use gositter_grammars::lex::LexOps;
use lexmock::MockLexer;

const OFFSET: u64 = 14695981039346656037;
const PRIME: u64 = 1099511628211;

fn fold(vals: &[u64]) -> u64 {
    vals.iter().fold(OFFSET, |h, v| (h ^ v).wrapping_mul(PRIME))
}

fn nothing<L: LexOps>(_: &mut L, _: u16) -> bool {
    false
}

fn adv<L: LexOps>(l: &mut L, _: u16) -> bool {
    l.advance(false);
    true
}

fn skp<L: LexOps>(l: &mut L, _: u16) -> bool {
    l.advance(true);
    true
}

#[test]
fn known_answers() {
    // result symbol, return value, position, mark
    assert_eq!(MockLexer::run(nothing, 0, &[]), fold(&[0, 0, 0, 0]));
    // advance: 1, position 0; then the final fold with ret = 1, pos = 1
    assert_eq!(MockLexer::run(adv, 0, &[7]), fold(&[1, 0, 0, 1, 1, 0]));
    assert_eq!(MockLexer::run(skp, 0, &[7]), fold(&[2, 0, 0, 1, 1, 0]));
}

#[test]
fn advance_and_skip_differ() {
    assert_ne!(MockLexer::run(adv, 0, &[7]), MockLexer::run(skp, 0, &[7]));
}

#[test]
fn mark_end_position_and_order() {
    let mark_then_adv = |l: &mut MockLexer, _: u16| {
        l.mark_end();
        l.advance(false);
        true
    };
    let adv_then_mark = |l: &mut MockLexer, _: u16| {
        l.advance(false);
        l.mark_end();
        true
    };
    let a = MockLexer::run(mark_then_adv, 0, &[7, 8]);
    let b = MockLexer::run(adv_then_mark, 0, &[7, 8]);
    assert_ne!(a, b);
    // mark_end mixes 3 and the position, then the final fold carries the mark
    assert_eq!(a, fold(&[3, 0, 1, 0, 0, 1, 1, 0]));
    assert_eq!(b, fold(&[1, 0, 3, 1, 0, 1, 1, 1]));
}

#[test]
fn result_symbol_counts() {
    let sym = |l: &mut MockLexer, _: u16| {
        l.set_result_symbol(5);
        false
    };
    assert_eq!(MockLexer::run(sym, 0, &[]), fold(&[5, 0, 0, 0]));
}

#[test]
fn eof_and_lookahead_past_the_end() {
    let probe = |l: &mut MockLexer, _: u16| {
        assert!(!l.eof());
        assert_eq!(l.lookahead(), 65);
        l.advance(false);
        assert!(l.eof());
        assert_eq!(l.lookahead(), 0);
        // at end of input advance does nothing, so the digest ignores it
        l.advance(false);
        l.advance(true);
        true
    };
    assert_eq!(MockLexer::run(probe, 0, &[65]), fold(&[1, 0, 0, 1, 1, 0]));
}

#[test]
fn empty_input_is_eof_with_lookahead_zero() {
    let probe = |l: &mut MockLexer, _: u16| l.eof() && l.lookahead() == 0;
    assert_eq!(MockLexer::run(probe, 0, &[]), fold(&[0, 1, 0, 0]));
}

#[test]
#[should_panic(expected = "loops at end of input")]
fn advance_at_end_panics_after_2_pow_20() {
    let spin = |l: &mut MockLexer, _: u16| {
        for _ in 0..=(1 << 20) {
            l.advance(false);
        }
        true
    };
    MockLexer::run(spin, 0, &[]);
}

static RUN_ABC: [bool; 256] = {
    let mut s = [false; 256];
    s[97] = true;
    s[98] = true;
    s[99] = true;
    s
};

#[test]
fn advance_run_equals_repeated_advance() {
    let run = |l: &mut MockLexer, _: u16| {
        l.advance_run(&RUN_ABC, false);
        true
    };
    let by_hand = |l: &mut MockLexer, _: u16| {
        while (0..128).contains(&l.lookahead()) && RUN_ABC[l.lookahead() as usize] && !l.eof() {
            l.advance(false);
        }
        true
    };
    let run_skip = |l: &mut MockLexer, _: u16| {
        l.advance_run(&RUN_ABC, true);
        true
    };
    let by_hand_skip = |l: &mut MockLexer, _: u16| {
        while (0..128).contains(&l.lookahead()) && RUN_ABC[l.lookahead() as usize] && !l.eof() {
            l.advance(true);
        }
        true
    };
    for input in [
        &[][..],
        &[97],
        &[97, 98, 99, 100, 97],
        &[100, 97],
        &[97, 98, 0x100],
    ] {
        assert_eq!(
            MockLexer::run(run, 0, input),
            MockLexer::run(by_hand, 0, input)
        );
        assert_eq!(
            MockLexer::run(run_skip, 0, input),
            MockLexer::run(by_hand_skip, 0, input)
        );
    }
    // stops at the first character outside the set
    let stops = |l: &mut MockLexer, _: u16| {
        l.advance_run(&RUN_ABC, false);
        l.lookahead() == 100
    };
    assert_eq!(
        MockLexer::run(stops, 0, &[97, 98, 100, 97]),
        fold(&[1, 0, 1, 1, 0, 1, 2, 0])
    );
}
