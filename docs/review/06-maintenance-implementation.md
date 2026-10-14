# Selected maintenance implementation

Implemented on 2026-10-14 after approval of **C1 and A01–A08, A12–A15, A17–A20**. The earlier numbered reviews remain historical evidence; their source line references describe the pre-implementation checkout.

## Changes

| Finding | Implementation |
| --- | --- |
| C1 / A20 | Boxed the large Rusoto error payloads with explicit conversions, preserving display and error sources. Removed hostname/string-key maps and the global credential-keyed pool cache. Shared typed connection/tool/SQL/output helpers replace repeated dispatch/rendering code. |
| A01 | Redacted credential `Debug`; removed explicit password and SQL-output logs; dependency debug/trace logs are disabled even at `-vv`. Generated login passwords are emitted once as JSON on stdout after successful creation. Unix log files are private and symlink-safe. |
| A02 | Read-only, ordered passfile parsing with comments, wildcards, escaping and database/user/port matching, explicit `PGPASSFILE`, conventional platform paths and explicit credential precedence. Source/destination credentials are resolved separately. Children use private, invocation-owned passfiles; inherited libpq service/password overrides are neutralized. |
| A03 | Added `--pg-ssl-mode`/`PG_SSL_MODE` and `--pg-ssl-root-cert`/`PG_SSL_ROOT_CERT`. Per the follow-up compatibility request, v2 defaults to `prefer`: unverified TLS when supported and non-TLS fallback. An implicit-mode warning announces the planned `verify-full` default in v3.0; an explicit CLI/environment mode suppresses it. Verified `verify-full`, encryption-required `require` and unencrypted `disable` remain available. Child clients receive the same selected mode/CA material and do not prefer GSS encryption over it. |
| A04 | SQL restores use `psql -X --set=ON_ERROR_STOP=1`; custom restores use `--exit-on-error`. Failed child statuses are errors. Full child SQL/stdout/stderr are not echoed into logs; errors report status rather than potentially sensitive SQL/data. Staging isolates ordinary restore failures. |
| A05 | Added target/owner/privilege/archive preflight, rejection of self-clone using configured endpoints, cluster/instance identity where available and resolved client addresses for common aliases, and a required database-update owner. Clone/restore populate staging DBs before publication. Overwrite renames old/staging databases transactionally; old cleanup failure reports a retained recovery name. |
| A06 | One identifier validator/quoting helper and escaped SQL literal helper cover role/database/schema/extension/catalog names and punctuation-rich passwords. Values in lookups remain bound parameters. libpq dbnames are explicitly quoted conninfo values, preventing a name containing `=` from becoming connection overrides. |
| A07 | Private per-run temporary directories; safe encoded filenames and a minimal file-name map; atomic dump/raw-copy/archive/download publication. Download filenames never share a global basename. Extraction selection excludes links/paths outside the workspace. |
| A08 | Ownership updates run sequentially in a transaction across application schemas, relations, sequences, types/domains, routines, large objects and the database. System schemas/extension members are excluded; schema-qualified names are quoted. No detached mutation tasks remain. |
| A12 | Small/empty uploads use PUT. Multipart sizes respect the part-count/size bounds; parts stream in 64 KiB chunks and reopen/seek on retry. Transient requests retry with bounded backoff, part/completion errors attempt abort and retain the primary error. Added optional session-token support and shared client construction; retained Rusoto rather than performing an SDK migration. |
| A13 | One workspace-finalization helper handles success/failure and explicit retention. Cleanup cannot replace a primary failure. Failed staging DBs are cleaned up; backup also supports `--keep-temp`. |
| A14 | Help/completions/tool checks dispatch without passfile reading or credential prompts. Noninteractive database operations do not attempt a terminal password prompt. |
| A15 | Checks only required clients, resolves their paths, validates process status/full version with a timeout, rejects mixed required-client majors, checks dump-client/server compatibility and custom-archive readability before staging. Administrative commands require no external client tools. |
| A17 | Removed external-input/I/O panic paths and hostname-map unwraps, uses OS temp locations and `Path` arguments, handles bare relative output parents, and guards Unix-specific permission APIs. Native Windows support/ACL validation remains a separate unimplemented platform task. |
| A18 | Format, strict Clippy, unit and maintenance regressions gate GitHub publishing jobs. Autofix applies rustfmt/Clippy suggestions, tests and commits changed Rust files to the actual source branch, then dispatches checks for that commit without packaging/publication. Fork/Dependabot PRs are excluded; pushes never force-update a branch. |
| A19 | Stable dynamic-build baseline, generated Debian ELF dependencies, mutually exclusive `/usr/bin` packages, static ELF checks, no demo secrets in distributable Docker ENV, accurate CI-provided Docker version labels, shared empty-list output contracts, descending fuzzy relevance, refreshed README and generated CLI reference. |

## Rust and release setup

Rust **1.99.0** is pinned for local builds (`rust-toolchain.toml`), GitHub checks/builds/formatting, Docker and the devcontainer. Cargo's minimum Rust version is 1.99. The edition remains 2021. Cargo.lock retains existing dependency versions; the implementation adds direct `libc` and `futures-util` dependencies already present transitively.

Dynamic GitHub release builds use Ubuntu 22.04 instead of a moving runner baseline. Static builds force native TLS/XZ static linkage and check for an ELF interpreter. `devops/package-deb.sh` generates dynamic dependencies with `dpkg-shlibdeps`, validates the static artifact, and builds packages in disposable staging directories. GitHub publication is configured, not invoked by this maintenance task.

The Ubuntu 22.04 baseline limits the dynamic binary's required glibc/OpenSSL/XZ versions; building on a newer host can raise those minimums. Main checks, autofix, the static-build Docker host and release publication use `ubuntu-latest`. Packaging uses the same distribution as the dynamic build so dependency discovery uses matching library metadata.

`${shlibs:Depends}` is a Debian substitution variable, not a shell variable. The packaging helper scans the dynamic ELF with `dpkg-shlibdeps` and replaces it with concrete package/version requirements before building the `.deb`. It covers linked runtime libraries, not Rust crates or the separately invoked PostgreSQL clients. The static package has no shared-library dependency placeholder.

Autofix no longer uploads an unused patch. With the repository token, bot pushes do not trigger normal push workflows, so it explicitly dispatches `ci.yml` with `checks_only=true`. That run checks the updated commit and skips preparation, packaging and publication. Repository branch protection still applies to bot pushes; no force push or bypass token is configured.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed without allowances/suppressions.
- `cargo test --locked --all-features`: **18 tests passed**, including boxed-error conversion/source preservation, ordered/escaped credential matching, redaction, private files, safe filename mapping, cleanup error precedence, empty output, multipart sizing/streams and the existing zstd/xz integrity tests.
- `cargo build --locked --release`: passed with Rust 1.99.0.
- `python3 tests/maintenance.py --docker --binary target/release/pg_actions`: all **eight end-to-end regression groups passed** against disposable PostgreSQL 18.6, generated local TLS certificates and a controllable HTTP S3 stub.
- YAML parsing, workflow shell syntax, publication dependency checks, packaging shell syntax and `git diff --check`: passed.
- Autofix follow-up: actionlint and Bash syntax passed; its exact commit script passed changed/unchanged branch and nested-Rust-file checks in a disposable local Git repository, excluding unrelated files. No GitHub push or workflow dispatch was performed locally.
- The final local Rust 1.99 Docker build passed its static ELF check; unprivileged `check-tools` detected the bundled client suite without credentials.

The original credential leak no longer reproduces in text/JSON verbose logs, including dependency-generated SQL traces. Ordered wildcard and escaped passfiles authenticate correctly without mutation, and explicit passwords override earlier wrong entries. Failing version commands are rejected. Invalid SQL/custom dumps, bad owners and self-clone are rejected without replacing the destination; valid clone/overwrite restores succeed. With `verify-full`, trusted TLS succeeds and absent TLS/untrusted CA/wrong hostname fail. The v2 `prefer` follow-up passed all eight groups with the default mode implicit, including non-TLS fallback, forced TLS with an untrusted certificate/mismatched hostname, Rust/child-client parity and warning suppression for explicit flags/environment. Both codecs, encoded database names and literal conninfo-like names restore correctly. Failed multipart parts/completion abort after bounded retries; successful small and multipart uploads complete.

Both dynamic and static release binaries passed the eight end-to-end groups. Both Debian packages built; their metadata/layout/conflicts were read back. The local dynamic package correctly declares its build-host libraries, including glibc >=2.39; the configured Ubuntu 22.04 release baseline is a separate CI build, not a claim about this local package. The static ELF has no interpreter.

Fixtures use their own containers, directories and mock endpoints and are removed after testing. No existing database/service/passfile was modified; no commit, push, registry publication or deployment was performed.

## Practical behavior changes and limits

The v2 default PostgreSQL mode is `prefer`, preserving non-TLS servers and unverified private/self-signed TLS connections. When the CLI/environment mode is implicit, database commands warn that v3.0 is planned to default to `verify-full`. Help/completions/tool checks remain warning-free. Select `PG_SSL_MODE=prefer` explicitly to retain this behavior without the migration warning, `verify-full` with a CA bundle for trusted TLS, `disable` for no TLS, or `require` to require encryption. Rust uses platform trust plus an optional bundle for `verify-full`; libpq uses its root policy plus that bundle, so private CAs should be supplied explicitly. A configuration-file override remains planned, not implemented.

Staging requires `CREATEDB`, suitable owner/rename privileges and additional DB disk space. Publication terminates active connections. Maintenance/template databases are not staging targets. Staging isolates trusted-dump failures; it does not sandbox arbitrary SQL/reconnection commands from an untrusted dump. Full child diagnostics are intentionally not written to logs because they can contain SQL/data.

Transaction boundaries are deliberately narrow: `CREATE DATABASE`, `DROP DATABASE`, connection termination and `psql`/`pg_restore` execution run outside explicit transactions. Ownership changes (`ALTER ... OWNER TO`) use one transaction. Publication uses a second transaction containing only `ALTER DATABASE ... RENAME TO`, issued while connected to `postgres`; a failed second rename rolls back the first. PostgreSQL forbids database creation/deletion and `ALTER DATABASE ... SET TABLESPACE` inside transaction blocks, but permits the ownership/rename forms used here. Restores are not passed `--single-transaction`. These paths passed the PostgreSQL 18.6 maintenance regressions; older server versions have not been executed locally.

References: [PostgreSQL ALTER DATABASE](https://www.postgresql.org/docs/16/sql-alterdatabase.html), [CREATE DATABASE](https://www.postgresql.org/docs/16/sql-createdatabase.html), [DROP DATABASE](https://www.postgresql.org/docs/16/sql-dropdatabase.html), [Debian dpkg-shlibdeps](https://manpages.debian.org/bookworm/dpkg-dev/dpkg-shlibdeps.1.en.html), and [GitHub workflow triggering](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).

TLS references: [PostgreSQL libpq SSL modes](https://www.postgresql.org/docs/current/libpq-ssl.html) and [tokio-postgres SSL modes](https://docs.rs/tokio-postgres/latest/tokio_postgres/config/enum.SslMode.html).

Multipart abort is best effort. A killed process, failed abort or lost create response still needs an object-store incomplete-upload lifecycle policy; no live lifecycle configuration was changed. S3 tests cover the real SDK request flow against a controllable stub, not a live production object store.

Native Windows execution, private Windows ACLs, actual hosted CI publication and the oldest supported distro ABI/runtime matrix were not exercised locally. Their workflows/support boundaries are stated explicitly rather than inferred from the current Linux build.

## Deferred scope

**A09, A10, A11, A16 and A21 remain deferred**, including cluster recovery/global objects, restore jobs, S3 URL precedence, general resource/cancellation limits and privilege-preset redesign. Existing privilege presets were only made safe to quote; their rights/default-owner/inheritance behavior was not changed. The file-name map added for A07 is not a cluster recovery manifest. The config-file, managed-tool download/bundling and complete Windows exploratory proposals were not implemented.
