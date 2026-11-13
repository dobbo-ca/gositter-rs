//! Port of gositter `grammars/rust/scanner.go` (tree-sitter-rust v0.24.2 `src/scanner.c`): the
//! scanner state, `serialize` and `deserialize`. `scan` waits for the runtime `Lexer` (Task 12).

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    opening_hash_count: u8,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.opening_hash_count;
        1
    }

    /// C resets, then reads only when the length is exactly 1.
    pub fn deserialize(&mut self, buf: &[u8]) {
        self.opening_hash_count = 0;
        if let [b] = buf {
            self.opening_hash_count = *b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TestSerializeRoundTrip
    #[test]
    fn serialize_round_trip() {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let n = Scanner::new().serialize(&mut buf);
        assert_eq!(&buf[..n], [0], "fresh scanner");
        for i in 0..=255u8 {
            let s = Scanner {
                opening_hash_count: i,
            };
            let n = s.serialize(&mut buf);
            assert_eq!(&buf[..n], [i], "serialize({i})");
            let mut got = Scanner {
                opening_hash_count: i.wrapping_add(1),
            };
            got.deserialize(&buf[..n]);
            assert_eq!(got, s, "round trip {i}");
        }
    }

    // TestDeserializeOtherLengths
    #[test]
    fn deserialize_other_lengths() {
        // C resets, then reads only when length is exactly 1.
        let cases: [&[u8]; 4] = [&[], &[], &[7, 7], &[7, 7, 7]];
        for b in cases {
            let mut s = Scanner {
                opening_hash_count: 5,
            };
            s.deserialize(b);
            assert_eq!(s.opening_hash_count, 0, "deserialize({b:?})");
        }
    }
}
