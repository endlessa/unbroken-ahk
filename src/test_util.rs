//! Shared helpers for the crate's own tests.

use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);

/// Fresh, hermetic storage directory: removed at start so no state leaks
/// between cargo-test invocations, and unique per call so parallel tests
/// never share a run counter. The sequence resets each process, so the
/// same names are reused run-to-run and /tmp litter stays bounded.
pub fn temp_storage_dir(prefix: &str) -> String {
    let dir = format!(
        "/tmp/unbroken-{}-{}",
        prefix,
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let _ = std::fs::remove_dir_all(&dir);
    dir
}
