//! Public-API regression test.
//!
//! Compares `core/PUBLIC-API.txt` against the live output of `cargo
//! public-api`. The test is `#[ignore]`d by default because:
//!
//! - It shells out to `cargo public-api`, which requires the
//!   `cargo-public-api` binary to be installed on the host.
//! - It requires a nightly toolchain to produce the rustdoc-json the
//!   tool consumes (`RUSTC_BOOTSTRAP=1` works for stable in CI).
//!
//! CI runs it explicitly:
//!
//! ```bash
//! cargo test -p fugle-marketdata-core --all-features \
//!     --test public_api_snapshot -- --ignored
//! ```
//!
//! When the snapshot drifts, regenerate via:
//!
//! ```bash
//! cargo public-api -p fugle-marketdata-core --simplified \
//!     > core/PUBLIC-API.txt
//! ```
//!
//! Then add an acknowledgement entry to `core/PUBLIC-API.md` describing
//! the change. See `core/PUBLIC-API.md` for the full workflow.

use std::path::PathBuf;
use std::process::Command;

const SNAPSHOT_FILENAME: &str = "PUBLIC-API.txt";

#[test]
#[ignore = "requires cargo-public-api binary; CI runs explicitly"]
fn public_api_matches_snapshot() {
    let snapshot_path = snapshot_path();
    let expected = std::fs::read_to_string(&snapshot_path).unwrap_or_else(|e| {
        panic!(
            "failed to read snapshot at {}: {e}\n\
             Regenerate with `cargo public-api -p fugle-marketdata-core \
             --simplified > core/PUBLIC-API.txt`",
            snapshot_path.display()
        )
    });

    let actual = run_cargo_public_api();

    if expected.trim() != actual.trim() {
        panic!(
            "Public API surface drifted from snapshot.\n\n\
             Expected (from {}):\n{}\n\n\
             Actual:\n{}\n\n\
             If the change is intentional, regenerate the snapshot and add an \
             acknowledgement entry to core/PUBLIC-API.md.",
            snapshot_path.display(),
            expected,
            actual,
        );
    }
}

fn snapshot_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join(SNAPSHOT_FILENAME)
}

fn run_cargo_public_api() -> String {
    let output = Command::new("cargo")
        .args([
            "public-api",
            "-p",
            "fugle-marketdata-core",
            "--simplified",
            "--all-features",
        ])
        .output()
        .expect(
            "failed to invoke `cargo public-api`. \
             Install via `cargo install cargo-public-api --locked` and retry.",
        );

    if !output.status.success() {
        panic!(
            "`cargo public-api` failed (status {}):\nstdout:\n{}\n\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    String::from_utf8(output.stdout).expect("`cargo public-api` produced non-UTF-8 output")
}
