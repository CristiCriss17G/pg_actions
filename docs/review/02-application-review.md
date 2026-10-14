# Application maintenance review

Reviewed on 2026-10-14. This is the **historical pre-implementation review**. The subsequently approved A01–A08, A12–A15 and A17–A20 changes are tracked in [06-maintenance-implementation.md](06-maintenance-implementation.md); A09/A10/A11/A16/A21 remain deferred. Findings are grouped by shared cause rather than repeated at each caller, and source references below describe the original checkout.

Scope: all current Rust modules, Cargo configuration, README, Docker/Compose, Debian packaging, GitHub workflows, and test/build scripts. `Legacy/` is historical and not part of the current binary; its contents were not audited as another application. Secret files and credential values were not inventoried. This is a code/execution review, not a full penetration test or dependency vulnerability scan.

Evidence labels: **code** means the behavior follows from the inspected implementation; **probe** means reproduced locally without contacting a database or S3. Proposed tests are future acceptance criteria, not tests that were run.

## Priorities

| Priority | Findings | Reason |
| --- | --- | --- |
| High | A01–A08, A18 | Credential exposure, connection trust, incorrect success, partial/destructive operations, unsafe file handling, or publication before test success |
| Medium | A09–A17, A21 | Backup completeness, reliability, compatibility, privileges and operational behavior |
| Low | A19–A20 | Output contracts and structural maintenance |

Only **A20/C1** coincides with the actual Clippy output. Other findings were discovered during task 2 and should not be treated as automatic Clippy fixes.

## A01 — Credentials and restored SQL enter logs

**Evidence (code):** `src/main.rs:222` traces the whole credential map; `PostgresCredentials` derives `Debug` with its password field (`structs.rs:24`). `db/general.rs` traces credentials during passfile merging and pool lookup/creation. `src/utils/user.rs` debug-logs supplied passwords on create/update; `db/user.rs:144` logs generated passwords at INFO. SQL restore enables `psql -e` and logs its stdout at DEBUG (`db/general.rs:396` onward).

**Consequence:** normal user creation can put a generated password in stderr and the optional log file. Verbose operations expose stored passwords and potentially SQL/data from a restore. `hide_env_values` in Clap hides help output, not these logs. File logging has no explicit private-file permission policy (`logging.rs:146`).

**Proposal:** give credentials a redacted `Debug`, remove secret-bearing log arguments, replace password logs with an explicit one-time credential delivery mechanism, avoid echoing full SQL by default, and create sensitive logs privately. Apply redaction centrally and at explicit password log sites. Test all verbosity levels and file/JSON logging with sentinel secrets. This is a prerequisite for [secure configuration](03-configuration.md).

## A02 — Passfile parser, lookup and persistence need one redesign

**Evidence (code):** `db/general.rs:36–143` always reads/writes `~/.pgpass`, splits each trimmed line on every `:`, parses ports with `expect`, ignores the database column, and stores one entry per hostname in a `HashMap`. It does not implement comments, wildcard fields, escaping, first-match order, `PGPASSFILE`, or the Windows passfile location. The main resolver uses a host match before considering explicit CLI/environment credentials (`main.rs:171–210`). Clone likewise keeps existing destination-host credentials instead of honoring supplied destination overrides (`clone.rs:83–117`).

**Evidence (probe):** a disposable passfile containing a legal wildcard port (`localhost:*:*:fixture:fixture-only`) caused `check-tools` to panic with exit 101. No user's passfile was read or modified in this probe.

**Consequence:** two servers on the same host with different ports/users overwrite each other; CLI overrides can silently be ignored; valid libpq passfiles can panic or misparse. `init_pgpass` truncates and rewrites the user's file with wildcard database entries and no escaping, losing comments/database scope and potentially reordering conflicting entries. Concurrent invocations can race. Explicit passwords can persist to disk without a separate request. Rust connections and child tools can use different credentials, particularly with an inherited `PGPASSFILE`/`PGPASSWORD`.

**Proposal:** resolve one connection tuple `(host, port, database, user)` with defined precedence, parse libpq's grammar/order, and treat an existing user passfile as read-only input. Give each invocation a private temporary passfile for children, set `PGPASSFILE` explicitly, and neutralize conflicting inherited connection/password variables. Create it with private permissions from the start, escape values, and retain an owning guard until all children exit. Make permanent credential storage explicit. Test duplicate hosts, ports/users/databases, wildcards, escapes, comments, CLI precedence and concurrent runs. Shared work with all three exploratory reports.

## A03 — PostgreSQL connection trust is weakened

**Evidence (code):** `db/general.rs:176–180` unconditionally calls `danger_accept_invalid_certs(true)`. Database pool configuration provides no user-selectable TLS mode or CA path; child `pg_*` processes receive no corresponding TLS policy from the application.

**Consequence:** an encrypted Rust connection does not establish certificate trust; there is no single connection-security policy shared with the subprocesses. The inspected `tokio-postgres 0.7.18` defaults to `SslMode::Prefer`, and the application does not override it: a non-TLS server is therefore not categorically refused. This does not mean every current connection is plaintext. [tokio-postgres SSL modes](https://docs.rs/tokio-postgres/latest/tokio_postgres/config/enum.SslMode.html).

**Proposal:** expose explicit TLS settings and CA configuration, verify server identity for remote connections, and propagate an equivalent policy to libpq children. Keep insecure/self-signed acceptance explicit and deliberate. Test a trusted certificate, untrusted certificate, wrong hostname and a server without TLS. Coordinate with configuration profiles rather than changing connection defaults silently in this sweep.

## A04 — Plain SQL restore can report success after SQL errors

**Evidence (code):** `db/general.rs:386–459` runs `psql -e --file=...` without `ON_ERROR_STOP=1` or `-X`; success is based only on the process exit status. Startup files can alter behavior. Custom restores also lack `--exit-on-error`.

**Consequence:** psql's ordinary continue-after-error behavior can leave an incomplete database while the CLI logs restore success. Custom restore normally reports errors through its exit status, but continues applying statements before that failure, leaving partial results. [psql error handling](https://www.postgresql.org/docs/current/app-psql.html#APP-PSQL-VARIABLES).

**Proposal:** use `psql -X --set=ON_ERROR_STOP=1`, add `pg_restore --exit-on-error`, retain bounded stderr error context and define a transactional mode where the dump permits it. Test a syntax/constraint error in the middle of a SQL dump, a failing custom restore and a conflicting startup file. Transactions/parallelism need explicit compatibility decisions, not unconditional flags.

## A05 — Destructive workflows lack complete preflight and recovery

**Evidence (code):** restore extracts and checks compression before dropping the old DB, which is good, but raw dump format/content and owner validity are not fully checked before `delete_db` (`restore.rs:125–173`). Clone does not reject identical source/destination endpoints and database names before overwrite (`clone.rs:179–212`). Database update with no `--new-owner` defaults to the connected superuser and reassigns objects (`database.rs:96–110`).

**Consequence:** a wrong raw dump or nonexistent/invalid requested owner can fail after the destination was deleted. Same-database clone with overwrite drops the source before reconstruction. A seemingly empty database update changes ownership. Drop/create/restore is not atomic and connections can reconnect after termination.

**Proposal:** build and validate an operation plan first: canonical connection identity, source/target inequality, owner existence, dump format/listing and target permissions. Require a meaningful update option. Offer staged restore into a new DB and deliberate cutover, or an explicit recovery-backup policy; describe remaining non-atomic steps. Test invalid dumps/owners and self-clone before any drop, and failure after creation. Distinguish this from A04's misleading SQL success.

## A06 — SQL identifier and literal handling is inconsistent

**Evidence (code):** `db/user.rs:257` and `:403` validate the role but interpolate unvalidated database/schema names into SQL. Prepared grant/revoke statements do the same. `alter_role` writes an unquoted role (`db/user.rs:218`), and table-owner changes write an unquoted owner (`db/database.rs:286`). Catalog table/type names are placed inside quotes without escaping embedded quotes. `validate_pg_names` (`db/general.rs:22`) accepts empty strings, hyphens and Unicode alphanumerics but is also used as a password validator.

**Consequence:** ordinary mixed-case/hyphenated role names fail or resolve incorrectly, legal catalog identifiers with quotes break queries, and unchecked SQL fragments can alter statement structure. Passwords with normal punctuation are rejected. Whitelisting some inputs does not fix all identifier construction sites.

**Proposal:** create a shared identifier-quoting function that doubles embedded quotes, validate emptiness/length separately, and use a dedicated safe literal mechanism for password DDL (ordinary bind placeholders are not universally valid in utility statements). Parameterize value queries, retain their existing safe bindings, and apply proper quoting to catalog-derived names too. Test spaces, quotes, hyphens, mixed case, Unicode and punctuation-rich passwords, including hostile schema/database inputs.

## A07 — Database names and remote filenames are unsafe filesystem keys

**Evidence (code):** backup joins database names directly into dump paths (`backup.rs:182`); clone joins `new_database` into a dump filename before `create_db` validates it (`clone.rs:132`); restore downloads to `/tmp/psql_backup/<remote-basename>` (`restore.rs:250–253`). These download names are not unique. `db_dump` opens files with create/truncate; downloads use create/truncate and set mode 0644 afterwards (`misc.rs:72–88`).

**Consequence:** quoted PostgreSQL names containing path separators can escape or collide with the intended file layout, and clone input can select a path before SQL validation runs. Simultaneous restores of the same basename overwrite/delete each other's archive. Existing symlinks in shared staging paths can redirect truncation. Downloaded backup data is world-readable by the application policy. A failed final retry can leave a partial download untracked because `temp_archive` is assigned only after success.

**Proposal:** use per-run private temporary directories and exclusive file creation; map DB names to safe filenames and record the original name in a manifest. Register temporary ownership before downloads, publish only complete downloads, and set private permissions at creation. Do not infer ownership of a file from its basename. Preserve intentional user output paths under a separate explicit overwrite policy. Test concurrent downloads, slash-containing names, malicious symlinks and failed stream copies. Shared with Windows temporary-path work.

## A08 — Ownership changes are incomplete and continue after early failure

**Evidence (code):** `db/database.rs:239–392` changes only tables in `public` and selected base/enum types, omits type namespaces in the ALTER statement, and spawns a task per object. It returns on the first task error while other spawned tasks remain detached. Clone/restore advertise owner reassignment through these helpers.

**Consequence:** sequences, views, functions/procedures, schemas and tables in other schemas can retain the restoring superuser's ownership. Types outside the search path can fail or target a same-named type. Mutations can continue after the helper reports failure. The backup task join loop already waits for all tasks; ownership helpers do not.

**Proposal:** define precisely which objects should change, enumerate schema-qualified identifiers, use correct quoting (A06), bound concurrency and await every task. Prefer a transaction for compatible object DDL. Evaluate `REASSIGN OWNED` only with care: its scope can include more than the restored objects and shared objects, so it is not a blind replacement. Test multiple schemas and object kinds, extension-owned objects and injected task failure.

## A09 — “all” backups are database dumps, not full cluster recovery

**Evidence (code):** `backup.rs:140–153` lists ordinary databases and dumps them individually. `db_dump` always passes `--no-owner --no-privileges` (`db/general.rs:275–276`), and `db_restore` uses the same flags. PostgreSQL ignores dump-time `--no-owner` for custom archives, so owner metadata can still be present there; the app's restore nevertheless skips applying it. No global-role/tablespace backup or original-owner manifest is produced. [pg_dump scope and ownership options](https://www.postgresql.org/docs/current/app-pgdump.html).

**Consequence:** data can restore while users, grants and original ownership are lost; a fresh server may lack required roles/extensions. Separately timed per-database snapshots are not a coordinated cross-database snapshot. The current behavior is useful for portable clones but is a weaker disaster-recovery contract than the name “all” may suggest.

**Proposal:** distinguish portable-data backup from recovery backup, document current omissions, optionally capture globals through a deliberately scoped `pg_dumpall --globals-only`, and add a manifest with DB names, owners, server/tool versions, formats and checksums. Preserve extension requirements and discuss cross-database consistency. Test restoration onto a clean server, not only onto the source server. Adding `pg_dumpall` affects [tool distribution](04-postgres-tools.md).

## A10 — Restore jobs flag currently has no effect

**Evidence (code):** `RestoreArgs.jobs` (`restore.rs:41`) is parsed but never passed to `db_restore`; the child has no `--jobs`. Unlike backup jobs, zero is accepted. Custom restore reads its archive from stdin (`db/general.rs:330–345`).

PostgreSQL requires a regular custom archive file or directory for parallel restore, and it cannot be combined with `--single-transaction`. [pg_restore jobs requirements](https://www.postgresql.org/docs/current/app-pgrestore.html).

**Proposal:** either remove/deprecate the misleading flag or implement it by passing an actual custom/directory archive filename to `pg_restore --jobs=N`; parallel restore needs a seekable supported archive, not the current stdin stream. Reject zero and define whether SQL format rejects or ignores the option explicitly. Test command construction and real parallel restore, keeping transaction restrictions in mind.

## A11 — S3 URLs do not get the documented precedence

**Evidence (code):** `restore.rs:225–249` validates all configured S3 fields, then uses global string `replace` to remove `s3://`, the configured bucket and prefix. `download_file` reconstructs the key with the configured prefix (`misc.rs:365–368`). The URI bucket is not parsed into the request. Help says the URL has priority.

**Consequence:** a URL specifying another bucket still targets the configured bucket; repeated prefix/bucket text inside a legitimate key is removed; bucket-root objects need an artificial prefix. `PathBuf` is being used to represent both a filesystem path and an object-store URL.

**Proposal:** parse a typed input enum (`Local(PathBuf)` or `S3 { bucket, key }`), preserve the key exactly, use explicit precedence and validate only fields required for the selected backend. Allow empty prefixes. Keep legacy abbreviated S3 input behind a documented compatibility path if needed. Test nested/repeated prefixes, another bucket, root keys and URL encoding.

## A12 — Multipart upload has no failure cleanup or large-file policy

**Evidence (code):** `misc.rs:179–325` creates an upload, sends fixed approximately 10 MiB parts sequentially and completes it. There is no abort request or bounded retry. The displayed part count uses 10,000,000 bytes while the buffer uses 10 × 1024 × 1024. Zero-length files lead to completion without parts. The part size never scales to object size.

**Consequence:** read/network/complete failure can leave paid multipart storage behind. Fixed part sizes eventually exceed S3's 10,000-part limit for large backups (approximately 97.7 GiB at the current 10 MiB size). Empty objects and transient failures lack a defined handling path. Endpoint capabilities may vary. [S3 multipart limits](https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html).

**Proposal:** use single PUT for small/empty objects and calculated multipart sizes within backend limits, retry retryable failures with bounded backoff, abort on failure and retain the original error if abort also fails. Set an incomplete-upload lifecycle rule as a second layer. Test stream/part/completion failures and size boundaries. Construct one reusable S3 client/validated settings object instead of duplicating setup for upload/download.

Both paths currently use `StaticProvider::new_minimal`, so the app has no session-token setting or standard credential-chain selection. Add those deliberately for temporary/role-based credentials. Upstream Rusoto describes itself as in maintenance mode; assess a maintained SDK when revisiting the client, including binary size, Rust baseline and S3-compatible endpoint behavior, rather than automatically swapping dependencies. [Rusoto project](https://github.com/rusoto/rusoto).

## A13 — Temporary cleanup is partial and can hide the real error

**Evidence (code):** restore repeats `cleanup(...).await?` before returning the original error, so cleanup failure replaces it. Ownership-error branches return without cleanup (`restore.rs:197–214`). Extraction uses `TempDir::keep` and manual path tracking (`restore.rs:299–305`). Clone removes its dump only on success, and an unsuccessful S3 upload leaves the backup archive.

**Proposal:** own temporary resources in one operation context, make retention an explicit state change, run final cleanup on every exit and preserve the primary failure while reporting cleanup failures separately. Keep failed artifacts only according to an intentional recovery policy and print their actual locations. Test permission/cleanup failures and owner-update failure. This shares resource ownership work with A07 but concerns lifecycle and error precedence rather than filename safety.

## A14 — Utility commands unnecessarily require database credentials

**Evidence (code):** `main.rs:171–210` resolves a passfile or prompts for a PostgreSQL password before dispatching every command, including completions and check-tools.

**Evidence (probe):** with an empty disposable home and no credential variables, `completions bash` and `check-tools` both exited 1 with no stdout in noninteractive execution. The failure happens before their utility work.

**Proposal:** dispatch commands that do not need a connection before credential resolution; make interactive prompting command-specific and fail clearly in noninteractive database operations. Test completions/check-tools on a fresh install without a database or passfile. This is essential for config init/show and tool installation commands proposed in the exploratory reports.

## A15 — Tool checks miss command failures and operation compatibility

**Evidence (code):** `misc.rs:456–511` reads version stdout without checking status, requires all three tools for every backup/clone/restore, and only enforces major >=16. It does not compare the dump client against the server or the restore client against the dump. The version commands use blocking `std::process::Command::output` without a timeout.

**Evidence (probe):** three explicit mock tool paths printed PostgreSQL 18.0 and exited 7; `check-tools` nevertheless returned 0 and logged “All tools are available.”

**Proposal:** validate exit status, report resolved absolute path/version per tool, check only needed tools for each operation, select a coherent suite and check server/archive compatibility before mutation. Move checks off async worker threads or use async children with timeouts. A “>=16” check is not a server-version compatibility guarantee. See [the distribution plan](04-postgres-tools.md).

## A16 — Process and resource limits are not explicit

**Evidence (code):** dump/restore children have no timeout or `kill_on_drop` policy; stderr read errors are logged then ignored (`db/general.rs:294–315`, `:364–382`). Sync file opens and compression detection happen in async functions, though heavy archive creation/extraction correctly use `spawn_blocking`. Extraction unpacks the whole archive without byte/file-count limits (`archive.rs:188–213`). Backup spawns one task per database and limits active dumps with a semaphore; ownership tasks have no similar bound.

**Proposal:** define optional operation/connect timeouts, propagate stream-read failures, guarantee child termination/reaping on cancellation, cap active object work and add extraction disk/file limits for untrusted or unusually large input. Retain library path protections rather than assuming `tar::unpack` is automatically a traversal vulnerability. Test cancellation, a hanging child, stderr failure, a large archive and disk-full behavior. Long database work may legitimately need no short default timeout.

## A17 — Recoverable failures still panic; paths assume Unix

**Evidence (code):** passfile home/port/file-open handling uses `expect`/panic; S3 metadata and credential fields use `expect`/`unwrap`; SQL restore converts a path with `to_str().unwrap`; credential maps are accessed with `unwrap` at dispatch sites. `misc.rs:13` imports Unix permissions unconditionally, and backup/clone/restore hardcode `/tmp/psql_backup`. `check_file_directory_path_exists` checks the parent even when used to validate a directory (`misc.rs:117`); for a bare relative output filename its empty parent does not exist.

**Consequence:** missing home, malformed passfiles, non-UTF-8 paths or inconsistent settings can crash; a simple relative backup filename can be rejected; Windows cannot compile the Unix import.

**Proposal:** use typed errors for external input/I/O, pass `OsStr` paths directly to children, centralize validated connections/tools and use OS temporary directories. Separate “directory exists” from “parent exists” and treat an empty parent as the current directory. Release profile has `panic = "abort"`, so do not rely on unwinding to recover resources. Coordinate with [Windows support](05-windows.md).

## A18 — Release publication does not wait for tests; autofix is fragile

**Evidence (code):** `.github/workflows/ci.yml` starts tests, prepare, Docker and standalone jobs, but publishing workflows depend only on prepare. A failed tests job does not gate Docker pushes or the standalone release. `autofix.yml` runs on PRs and pushes but always uses `github.head_ref` for the push destination; it is empty for push events. It runs `cargo fmt` before `cargo clippy --fix` without `--allow-dirty`, and demands `-D warnings` even though C1 has no machine-applicable fix. Its stable toolchain differs from the 1.96 build/test pin.

**Proposal:** require successful tests/lint before publication, choose a deliberate formatter/lint toolchain, make autofix a reviewed patch/PR workflow or handle branch push events correctly, and account for cargo-fix dirty-tree rules. A18 is distinct from C1: fixing C1 alone does not repair the publication dependencies or branch handling. Validate on a throwaway branch and PR before enabling writes; no workflow was triggered by this review.

## A19 — Packaging, documentation and output contracts drift

**Evidence (code):** the dynamic Debian control depends on `openssl`/`xz-utils` rather than generated ELF library dependencies and has no PostgreSQL-client recommendation. The static control still requires libc/xz; both install `/usr/local/bin` and use the same package name. Dynamic release builds on moving `ubuntu-latest`, while README advertises older distributions without an ABI verification matrix. Docker supplies demo PostgreSQL/S3 passwords in runtime ENV and has a stale fixed version label. README's between-host clone example uses `--pg-hostname` rather than `--pg-hostname2`. User/database/extension renderers duplicate formatting; database lists emit no JSON/CSV when empty, unlike the other list paths. Fuzzy results sort scores ascending (`db/general.rs:462–471`) rather than documenting a relevance order.

**Proposal:** verify ABI/static claims with `file`/`ldd`/`readelf`, build dynamic artifacts against the oldest supported baseline, generate Debian dependencies, clarify package alternatives and install locations, remove demo credentials from distributable defaults, and update examples from generated help. Specify stable machine-output schemas including empty lists and fuzzy relevance ordering, then share renderers where it preserves those contracts. These are multiple small product/distribution improvements grouped here because they require explicit compatibility/documentation choices. See A15 and the installation report for PostgreSQL dependency handling.

## A20 — Rust structure and errors

**Clippy overlap: C1.** The shared `PGCliError` contains three large Rusoto multipart error variants; all 114 diagnostics stem from that representation. See [01-clippy.md](01-clippy.md) for the grouped inventory and targeted boxing plan. Do not fix each caller separately.

Other structural improvements: replace string-keyed `HashMap`s for known tool fields with a validated tool-suite structure; use a resolved connection/profile object rather than hostname-keyed credential maps; use `&str` where ownership is unnecessary; consolidate repeated match/log/return and output code; preserve structured source errors rather than converting many to generic strings. Global pools keyed by credentials retain secret clones and can race between lookup and insertion (`db/general.rs:150–204`); for a short-lived CLI, consider operation-owned connections/pools instead of a process-wide cache. None of these justify a broad refactor before execution correctness is covered by tests.

## A21 — Privilege presets do not fully express their advertised intent

**Evidence (code):** `db/user.rs:257–546` applies grants/revokes statement by statement, changes default privileges without `FOR ROLE`, and grants `USAGE, SELECT` on existing sequences in its read-only preset. Some full-schema grants are repeated. Errors do not roll back earlier grants, and “revoke” is not a calculation of effective privileges inherited through other roles or PUBLIC.

**Consequence:** future objects created by an application owner do not necessarily inherit defaults changed for the administrative account. Sequence USAGE allows `nextval`, so “read-only” is not a guarantee of zero writes. Removing one preset may affect access associated with another preset, while inherited rights can remain. [Default privilege scope](https://www.postgresql.org/docs/current/sql-alterdefaultprivileges.html), [GRANT semantics](https://www.postgresql.org/docs/current/sql-grant.html).

**Proposal:** define preset semantics precisely, expose/resolve the future object-creator role, apply schema-level changes transactionally where possible, and report effective rights separately from grants made by this invocation. Keep database-level changes and schema-level failure/recovery explicit. Test non-superuser creators, future tables/sequences, overlapping presets, role inheritance/PUBLIC and failure midway through a grant. This is an authorization/product behavior change and is intentionally report-only.

## Verification and test gaps

Performed: all seven existing unit tests passed, the debug binary built, formatting passed after two formatting-only changes, Clippy was collected/grouped, and four isolated utility probes reproduced A02/A14/A15. Probes used disposable child-process home directories and mock version tools; they never contacted PostgreSQL/S3 or changed user credentials.

Not performed: database/S3 integration runs, a Windows build, release/static/ABI verification, dependency vulnerability scanning, or live CI execution. `tests/archive-compose.py` already covers useful codec/data round trips and malformed compressed archives, but GitHub's tests job only runs Rust unit tests. `tests/tests.sh` is an old manual command collection with outdated syntax, no assertions and destructive DB examples; it should not be treated as a runnable regression suite.

Suggested implementation order: credential/passfile and SQL correctness (A01–A06); private files, cleanup, ownership and privilege correctness (A07–A08/A13/A21); backup/recovery contract and S3 reliability (A09–A12); utility/tool checks and platform paths (A14–A17); release/packaging/output and the central Clippy refactor (A18–A20). Add isolated subprocess tests plus disposable PostgreSQL/S3 fixtures for these changes, rather than connecting tests to arbitrary existing environments.
