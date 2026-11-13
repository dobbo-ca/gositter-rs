//! Port of gositter `grammars/internal/cchar`: the plain `char` of the C compiler that the
//! matching C build uses, so a scanner port can follow C where a scanner stores a code point in a
//! `char` and reads it back (bash, ruby heredoc ends; hcl's `CHAR_MAX` limits).
//!
//! The target list is gositter's build tag `linux && (arm64 || arm || ppc64 || ppc64le || riscv64
//! || s390x)`, hard-coded: `core::ffi::c_char` differs from it on some non-Linux targets. Go's
//! `linux` tag also matches android, and Go's ppc64 and ppc64le are both Rust's `powerpc64`.

/// `char` is unsigned: the Linux ABI of these targets. Signed on x86 and every darwin target.
pub const UNSIGNED: bool = cfg!(all(
    any(target_os = "linux", target_os = "android"),
    any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc64",
        target_arch = "riscv64",
        target_arch = "s390x"
    )
));

/// `CHAR_MAX`.
pub const MAX: i32 = if UNSIGNED { 255 } else { 127 };

/// A `char` read back as an int, as C's integer promotion gives it.
pub fn value(b: u8) -> i32 {
    if UNSIGNED { b as i32 } else { b as i8 as i32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // cchar_test.go TestValue
    #[test]
    fn value_follows_target() {
        let (want, max) = if UNSIGNED { (255, 255) } else { (-1, 127) };
        assert_eq!((value(0xFF), value(0x7F), MAX), (want, 127, max));
    }
}
