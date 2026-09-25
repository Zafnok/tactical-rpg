//! Pure, deterministic game rules. See ADR-0004.

// SCRATCH: temporary code to prove the mutation-testing gate (ticket 0105);
// removed before this PR merges.
#[doc(hidden)]
pub fn scratch_is_positive(x: i32) -> bool {
    x > 0
}

#[cfg(test)]
mod scratch_tests {
    use super::*;

    #[test]
    fn scratch_is_positive_runs() {
        let _ = scratch_is_positive(5);
    }
}
