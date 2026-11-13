//! gositter `export_test.go`: hooks into the runtime's internals for the integration tests
//! and the fuzz targets. Not part of the API.

use crate::Language;

impl Language {
    /// Compares the comb small-table lookup with the C scan for every small state and every
    /// symbol value a lookup can carry: (state, symbol, built, ok) of the first difference.
    #[doc(hidden)]
    pub fn dense_mismatch(&self) -> (u32, u32, bool, bool) {
        todo!("gositter-rs-2f0.7")
    }

    /// The size of the derived small parse table in bytes.
    #[doc(hidden)]
    pub fn small_table_bytes(&self) -> usize {
        todo!("gositter-rs-2f0.7")
    }

    /// Compares `field_id_for_name` with the C scan on every field name, each of its
    /// prefixes, and each with "x" or "\0" added; the first name that differs.
    #[doc(hidden)]
    pub fn field_id_mismatch(&self) -> Option<String> {
        todo!("gositter-rs-2f0.7")
    }
}
