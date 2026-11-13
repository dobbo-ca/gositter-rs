//! What a generated `lex.rs` calls. `LexOps` is the lexer interface of `lex` and `lex_keywords`,
//! so the runtime `Lexer` and the oracle's `MockLexer` both drive the same generated code.
//! `run_table` interprets the table form (`lex.bin`, format in `gengrammar/src/lexgen.rs`).

/// The lexer methods a generated lex function uses (gositter `lexer.go`).
pub trait LexOps {
    fn lookahead(&self) -> i32;
    fn eof(&self) -> bool;
    fn advance(&mut self, skip: bool);
    /// `advance(skip)` while the lookahead is an ASCII character in `set`.
    fn advance_run(&mut self, set: &'static [bool; 256], skip: bool);
    fn mark_end(&mut self);
    fn set_result_symbol(&mut self, sym: u16);
}

impl LexOps for gositter::Lexer<'_> {
    #[inline]
    fn lookahead(&self) -> i32 {
        self.lookahead
    }
    #[inline]
    fn eof(&self) -> bool {
        gositter::Lexer::eof(self)
    }
    #[inline]
    fn advance(&mut self, skip: bool) {
        gositter::Lexer::advance(self, skip)
    }
    #[inline]
    fn advance_run(&mut self, set: &'static [bool; 256], skip: bool) {
        gositter::Lexer::advance_run(self, set, skip)
    }
    #[inline]
    fn mark_end(&mut self) {
        gositter::Lexer::mark_end(self)
    }
    #[inline]
    fn set_result_symbol(&mut self, sym: u16) {
        self.result_symbol = sym;
    }
}

/// C `set_contains`: binary search over `(start, end)` ranges, as `gengrammar/src/cond.rs`.
pub fn set_contains(ranges: &[(i32, i32)], lookahead: i32) -> bool {
    let (mut index, mut size) = (0, ranges.len());
    if size == 0 {
        return false;
    }
    while size > 1 {
        let half = size / 2;
        let mid = index + half;
        let (start, end) = ranges[mid];
        if (start..=end).contains(&lookahead) {
            return true;
        } else if lookahead > end {
            index = mid;
        }
        size -= half;
    }
    (ranges[index].0..=ranges[index].1).contains(&lookahead)
}

/// `set_contains` with the answers for code points below 128 precomputed:
/// `ascii[c] == set_contains(ranges, c)`.
#[inline]
pub fn set_contains_ascii(ascii: &[bool; 128], ranges: &[(i32, i32)], lookahead: i32) -> bool {
    if (0..128).contains(&lookahead) {
        ascii[lookahead as usize]
    } else {
        set_contains(ranges, lookahead)
    }
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn i32_at(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Whether `lookahead` is in the `n` sorted, disjoint `(i32, i32)` ranges at `off`.
fn in_ranges(data: &[u8], off: usize, n: usize, lookahead: i32) -> bool {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let (start, end) = (i32_at(data, off + 8 * mid), i32_at(data, off + 8 * mid + 4));
        if lookahead < start {
            hi = mid;
        } else if lookahead > end {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// The table-form lexer: runs the state machine of `data` (one function's region of `lex.bin`,
/// format in `gengrammar/src/lexgen.rs`) with the calls of the code form. `runs` are the run
/// sets that the records index.
pub fn run_table<L: LexOps>(
    data: &'static [u8],
    runs: &'static [&'static [bool; 256]],
    l: &mut L,
    mut state: u16,
) -> bool {
    let mut result = false;
    'state: loop {
        let (mut lookahead, mut eof) = (l.lookahead(), l.eof());
        // the directory is sorted by state
        let (mut lo, mut hi) = (0, u16_at(data, 0) as usize);
        let mut rec = None;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let d = 2 + 6 * mid;
            let s = u16_at(data, d);
            if state < s {
                hi = mid;
            } else if state > s {
                lo = mid + 1;
            } else {
                rec = Some(u32::from_le_bytes([
                    data[d + 2],
                    data[d + 3],
                    data[d + 4],
                    data[d + 5],
                ]));
                break;
            }
        }
        let Some(mut p) = rec.map(|r| r as usize) else {
            return false;
        };
        let run = u16_at(data, p) as usize;
        if run != 0xFFFF && (0..128).contains(&lookahead) && runs[run][lookahead as usize] {
            l.advance_run(runs[run], data[p + 2] != 0);
            (lookahead, eof) = (l.lookahead(), l.eof());
        }
        p += 3;
        loop {
            let tag = data[p];
            p += 1;
            match tag {
                0 => return result,
                1 => {
                    result = true;
                    l.set_result_symbol(u16_at(data, p));
                    l.mark_end();
                    p += 2;
                }
                _ => {
                    let flags = data[p];
                    let target = u16_at(data, p + 1);
                    let (mut off, mut n) = (p + 5, u16_at(data, p + 3) as usize);
                    p = off + 8 * n;
                    if flags & 2 != 0 {
                        let n_eof = u16_at(data, p) as usize;
                        if eof {
                            (off, n) = (p + 2, n_eof);
                        }
                        p += 2 + 8 * n_eof;
                    }
                    if in_ranges(data, off, n, lookahead) {
                        l.advance(flags & 1 != 0);
                        state = target;
                        continue 'state;
                    }
                }
            }
        }
    }
}
