//! Shared helpers for the crate's own tests.

use std::sync::atomic::{AtomicU32, Ordering};

static SEQ: AtomicU32 = AtomicU32::new(0);

/// Fresh, hermetic storage directory: removed at start so no state leaks
/// between runs, unique per call so parallel tests never share a run
/// counter, and namespaced by process id so two concurrent cargo-test
/// invocations can never delete each other's live storage. The pid
/// component means old directories accumulate until /tmp is cleaned —
/// an accepted cost of cross-process safety.
pub fn temp_storage_dir(prefix: &str) -> String {
    let dir = format!(
        "/tmp/unbroken-{}-{}-{}",
        prefix,
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let _ = std::fs::remove_dir_all(&dir);
    dir
}
