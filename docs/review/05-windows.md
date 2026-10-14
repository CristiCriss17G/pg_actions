# Native Windows feasibility

**Snapshot note:** this investigation predates the selected maintenance changes. Shared credential/tool/path foundations now exist; see [implementation status](06-maintenance-implementation.md). The exploratory feature itself remains deferred.

Investigated on 2026-10-14. **Feasible, but the present checkout is not Windows-ready. No Windows build or execution was performed.** Only the Linux x86_64 target is installed in this environment. This conclusion combines source inspection and upstream platform documentation, not a claimed successful port.

## Recommended first target

Start with a native **x86_64-pc-windows-msvc** executable and a portable ZIP, tested on Windows 11 and a selected Windows Server environment. This Rust target has Tier 1 host-tool support. Using a native Windows CI runner avoids the unnecessary complexity of cross-compiling C dependencies and linking MSVC artifacts from Linux. ARM64/32-bit artifacts can follow actual demand and a validated PostgreSQL-client supply. [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html).

The app's core is not inherently Linux-specific: async PostgreSQL/S3 clients, Clap, ordinary subprocess execution, path types and Rust archive handling are suitable foundations. It does not invoke Bash to run dumps/restores. The main blockers are OS assumptions and distribution/testing gaps, rather than replacing the execution model.

## Blockers and method

| Area | Current evidence | Required change |
| --- | --- | --- |
| Compilation | `misc.rs:13` imports `std::os::unix::fs::PermissionsExt` unconditionally and uses `Permissions::from_mode` | Put Unix mode handling behind `cfg(unix)` and implement a Windows private-file policy |
| Temporary files | Backup/clone/restore hardcode `/tmp/psql_backup`; restore logs that exact path | Centralize invocation-owned `tempfile` directories using the OS temp root, with a configurable override |
| Passfile location | `db/general.rs:36` and `:99` use `home_dir()/.pgpass` | Honor explicit `PGPASSFILE` and implement Windows's conventional passfile location; stop rewriting user files |
| Credential identity | Credentials keyed only by hostname; source/destination CLI credentials can be ignored | Shared tuple/profile resolver from maintenance A02 and config proposal |
| File names | Database names become filenames; remote basenames are reused | Safe name mapping/manifest and exclusive per-run files; handle Windows reserved names and case collisions |
| SQL paths | SQL restore builds `--file=` from `to_str().unwrap()` | Pass a separate `--file` argument followed by the original `Path`/`OsStr` |
| Tool discovery | Only explicit tool paths or PATH, minimum 16 | Shared suite resolver plus installed/bundled/managed Windows tools and version checks |
| Release | CI builds only Linux standalone/static/Debian assets | Add Windows build/test/package jobs and a Windows ZIP asset |
| Integration tests | Python suite isolates Linux HOME and relies on external codecs/tar; shell scripts use Bash | Inject explicit test paths/passfiles and make native Windows fixture/codec checks available |

### Permissions and file lifecycle

Unix mode 0600 is not a Windows ACL. Do not replace it with a no-op or `set_readonly(false)` and claim secret privacy. Use a private directory owned by the current user and a restricted/inherited DACL or explicit Windows security APIs for sensitive files; test the effective ACL. Keep this behind a small platform abstraction, shared by config/secret files, temporary passfiles and dumps.

Keep downloaded artifacts and file handles owned by the operation, close handles before rename/delete and account for Windows sharing violations, antivirus interference and inability to remove some open files. Preserve the main operation error if cleanup fails (A13). Use a directory guard instead of reconstructing a temp path manually. Release's `panic = "abort"` still means crashes can leave artifacts; operation correctness should use ordinary errors and deliberate retention/cleanup.

### Passfiles

PostgreSQL specifies `%APPDATA%\postgresql\pgpass.conf` on Windows and supports an explicit `PGPASSFILE`. It does not apply Unix permission checks there, so the application must separately ensure its generated secret files are private. The existing custom parser still needs wildcard/order/escape support; only changing the path will not fix credentials. [PostgreSQL passfile specification](https://www.postgresql.org/docs/current/libpq-pgpass.html).

Preferred integration: read an existing user's passfile as input, resolve a connection profile, then give child tools a separate per-invocation private passfile via `PGPASSFILE`. Keep source/destination entries distinct and eliminate conflicting inherited `PGPASSWORD`. Tests should inject config/temp/passfile roots explicitly: `dirs::home_dir` uses the Windows Known Folder API, so changing only `HOME` does not provide the same isolation as on Linux.

### Filename and subprocess behavior

Use `PathBuf` for local paths and a separate typed S3 URL; do not treat drive-letter paths as URLs or object keys. Validate/mapping must cover reserved names such as `CON`, characters such as `:`/`?`, trailing dots/spaces and case-insensitive collisions. Database identifiers can legally contain characters that are not safe filenames on Windows. Preserve original identifiers in a manifest rather than forcing all database names into a narrower naming policy.

Execute absolute tool paths directly with `Command`; pass paths/flags as separate arguments and let Rust handle Windows process argument encoding. Test `C:\Program Files\...`, Unicode directories, UNC paths and long paths on supported hosts. Keep raw custom dumps on binary file handles; do not convert them to text. Add cancellation/termination behavior and robust stderr capture (A04/A16), including Ctrl+C checks on Windows.

## Dependencies

| Dependency group | Evidence | Implication |
| --- | --- | --- |
| TLS (`native-tls`, Postgres connector, Rusoto default TLS) | `native-tls 0.2.18` has a Windows SChannel backend; the local Cargo dependency tree routes these users through it | The Rust app does not inherently need OpenSSL installed on Windows; PostgreSQL child-tool libraries are a separate matter |
| XZ (`xz2 0.1.7` → `lzma-sys 0.1.20`) | Inspected installed `lzma-sys` build script checks for MSVC and builds bundled C code instead of probing pkg-config | A native C build toolchain is required; this is encouraging source evidence, not proof the full app builds |
| Zstd (`zstd` → `zstd-sys`) | Inspected installed build script has MSVC branches and C compilation | Build and exercise both ordinary and parallel compression on the Windows runner |
| Tokio/Clap/Serde/tempfile/dirs | Existing implementation uses portable abstractions in most places | Preserve those abstractions and verify the project's full locked dependency graph on Windows |

Upstream `native-tls` explicitly documents SChannel and notes that vendored OpenSSL has no effect on Windows/macOS. This avoids a needless TLS-library migration purely to obtain Windows support. It does not fix the app's disabled certificate validation or make libpq use SChannel. [native-tls backends](https://docs.rs/native-tls/latest/native_tls/).

The local native dependency evidence is from the exact cached lockfile versions, not a successful cross-build. Do not replace XZ with another crate or remove backward archive support before trying the existing dependency on a native runner. A separate future SDK/dependency modernization decision is possible, but not a prerequisite proven by this review.

## PostgreSQL clients on Windows

A native PostgreSQL client suite is available through the project's EDB-linked Windows binary/installer distribution. Preserve existing installed tools for the first port; then add an optional portable suite/managed download as described in [04-postgres-tools.md](04-postgres-tools.md). [Official Windows download options](https://www.postgresql.org/download/windows/).

A Windows release bundle must include the selected tools' DLL/runtime dependency closure, not just three `.exe` files. Validate on a machine without an installed PostgreSQL distribution and keep DLL discovery tied to private bundle directories. Installing a PostgreSQL server service should not be necessary just to run backup/restore clients. The app's use of native TLS does not remove the clients' own crypto/compression dependencies.

WSL or Docker is an existing alternative execution route for Windows users, but it requires that environment and different path/network/credential handling. Document it as an alternative rather than calling the existing Linux binary a native Windows release.

## Proposed implementation phases

1. **Portability foundation (medium):** fix Unix imports, centralize private temp/config/file handling, stop `.pgpass` rewrites, use OS-native paths, fix utility dispatch and safe subprocess argument construction. Share work with maintenance A02/A07/A13–A17 and the config/tool proposals.
2. **Native build and smoke tests (small/medium after phase 1):** add a Windows CI job using the project's pinned Rust 1.96 toolchain, MSVC C tools, `cargo build --locked --release --target x86_64-pc-windows-msvc`, tests, formatting/lint checks as appropriate and help/completion/check-tools probes. Do not trigger it externally as part of this report.
3. **Real operation tests (medium):** use a disposable PostgreSQL server accessible over TCP and a disposable S3-compatible endpoint, with no arbitrary user's service/passfile. Cover database/user/extension operations, clone, SQL/custom backup/restore, both archive codecs and parallel compression. Add failing restores, bad certs, paths with spaces/Unicode, concurrent downloads and cleanup errors.
4. **Release (medium):** package `pg_actions.exe`, license/docs/checksums into a platform-named ZIP; add optional versioned PostgreSQL suites. Gate publication on successful native tests. Consider signing and package-manager manifests after the portable artifact is stable. Windows signing certificates/package submissions are additional release work, not needed to establish that the code can run.

## Acceptance boundary

Do not advertise Windows support based only on compilation. Require a clean Windows host to run all primary operations, correct credential/passfile behavior, private generated files, no dependency on Bash/WSL, working codecs/temporary cleanup and explicit tool/version diagnostics. Verify release assets separately from CI's preinstalled tools. The evidence currently supports **a practical porting plan**, not a ready Windows executable or measured effort estimate.
