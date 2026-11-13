//! gositter `FuzzAdvanceRun`: byte 0 is `which`, byte 1 `start`, bit 0 of byte 2 `skip`, the rest
//! the source.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some((h, src)) = data.split_first_chunk::<3>() {
        assert!(gositter::advance_run_matches_reference(
            src,
            h[0],
            h[1],
            h[2] & 1 == 1
        ));
    }
});
