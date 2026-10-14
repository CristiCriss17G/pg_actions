# Clippy maintenance review

**Implementation update:** C1 is now fixed and strict Clippy passes on Rust 1.99.0. See [06-maintenance-implementation.md](06-maintenance-implementation.md). The diagnostics and line numbers below are the historical pre-fix inventory.

Reviewed on 2026-10-14. Scope: the current Rust application, all targets and all features; legacy scripts are not Rust targets. The working tree was clean at the start.

## Result

Clippy reports **one issue repeated at 57 distinct source locations**, emitted twice for the binary and binary-test targets: **114 warnings in total**. Every diagnostic is `clippy::result_large_err`. There were no independent harmless Clippy suggestions to apply. No lint was suppressed and no error handling was changed.

The separate formatting check found two `info!` calls in `src/utils/clone.rs` that rustfmt wanted on one line. Those formatting-only changes were applied immediately; they do not affect execution.

## C1 — Shared error enum is too large (deferred)

**Root cause:** `src/utils/structs.rs:93`, especially the three Rusoto multipart error variants at lines 109–113. Clippy reports that each payload contains at least 136 bytes. Because almost every operation returns the same `PGCliError`, the lint propagates to unrelated functions and closures. These are not 57 different error-handling bugs and do not mean those functions are each called too often.

**Impact:** unnecessary size in `Result` values and potentially async futures, stack use and copies. This is a maintainability/performance finding, not evidence of incorrect backup data or a measured performance bottleneck.

**Recommended implementation:** box the three large Rusoto payloads inside their existing enum variants, preserve their error messages and sources, and provide explicit `From<RusotoError<...>> for PGCliError` implementations that allocate the boxes. Simply changing `#[from] RusotoError<T>` to `#[from] Box<RusotoError<T>>` does not preserve conversions from the unboxed errors returned by Rusoto, so the existing `?` sites need the explicit conversions. Re-run Clippy to check for any remaining oversized variants before broadening the change.

**Alternative:** box the entire error in the shared `Result` alias. This changes more signatures, direct enum constructors and matches, and adds allocations for small errors too; it is less targeted.

**Why deferred:** this changes the central error representation and conversion contract. It belongs in a small deliberate refactor with error-conversion checks, rather than an automatic lint sweep.

**Validation for a future fix:** compile every target, exercise conversion/display/source preservation for all three multipart errors, run the existing tests and strict Clippy. A size check can verify the intended reduction; benchmark before promising a speed improvement.

**Overlap with maintenance task 2:** [A20 — Rust structure and errors](02-application-review.md#a20--rust-structure-and-errors). Resolve C1 centrally, not by editing or allowing the lint at each warning site.

## Grouped diagnostic inventory

Line numbers refer to the formatted checkout produced by this review. Each entry counts once per source location, including closures.

| File | Unique locations | Lines |
| --- | ---: | --- |
| `src/main.rs` | 1 | 147 |
| `src/utils/archive.rs` | 8 | 27, 67, 96, 119, 136, 139, 188, 191 |
| `src/utils/backup.rs` | 2 | 68, 87 |
| `src/utils/clone.rs` | 1 | 58 |
| `src/utils/database.rs` | 2 | 68, 121 |
| `src/utils/db/database.rs` | 10 | 9, 21, 40, 117, 146, 165, 180, 218, 239, 319 |
| `src/utils/db/extension.rs` | 3 | 12, 71, 84 |
| `src/utils/db/general.rs` | 7 | 36, 150, 206, 219, 231, 319, 386 |
| `src/utils/db/user.rs` | 8 | 10, 26, 106, 129, 179, 205, 264, 410 |
| `src/utils/extension.rs` | 2 | 46, 76 |
| `src/utils/logging.rs` | 1 | 130 |
| `src/utils/misc.rs` | 6 | 108, 122, 139, 179, 328, 456 |
| `src/utils/restore.rs` | 4 | 63, 219, 290, 325 |
| `src/utils/user.rs` | 2 | 114, 229 |
| **Total** | **57** | **One root cause** |

## Checks performed

Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`, Linux x86_64. CI pins Rust 1.96 for tests/builds but uses moving stable for autofix; this review does not claim a separate Rust 1.96 run.

| Check | Result |
| --- | --- |
| Initial `cargo clippy --all-targets --all-features --message-format=json` | Exit 0; 114 warnings, 57 unique locations, one lint |
| `cargo test --locked --all-features` | All 7 existing tests passed |
| Initial `cargo fmt --all --check` | Failed on the two formatting changes described above |
| `cargo fmt --all`, then `cargo fmt --all --check` | Passed |
| `cargo build --locked` | Passed |
| Final `cargo clippy --locked --all-targets --all-features --message-format=json` | Exit 0; same 114 warnings and 57 unique locations |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Exit 101; C1 remains, as expected |

The application review and feasibility investigations are report-only. No database operation, deployment, dependency update, commit or push was performed.
