//! SHA-256 (FIPS 180-4), for the check of each `parser.c` against `grammars.lock`.

/// The SHA-256 digest of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let _ = data;
    todo!()
}

/// Lower-case hex, as Go's `%x`.
pub fn hex(d: &[u8; 32]) -> String {
    let _ = d;
    todo!()
}
