# PostgreSQL database actions

`pg_actions` is a personal administrative CLI for PostgreSQL: clone, back up and
restore databases, manage users and privileges, and manage databases/extensions.

## Installation

Builds use **Rust 1.99.0**, pinned in `rust-toolchain.toml`. From source:

```bash
cargo build --locked --release
./target/release/pg_actions --help
```

Release assets are published through [GitHub releases](https://github.com/CristiCriss17G/pg_actions/releases):

- `pg_actions`: dynamically linked Linux x86_64 binary. The release pipeline targets
  Ubuntu 22.04+/Debian 12+ by building on Ubuntu 22.04 with the required
  OpenSSL/liblzma runtime libraries. Local builds inherit their build host ABI;
  Debian packages calculate their actual ELF dependencies during packaging.
- `pg_actions-static`: Linux x86_64 musl binary with native TLS/compression
  libraries linked in. Release builds check that it has no ELF interpreter.
- `pg_actions_<version>_amd64.deb` and
  `pg_actions-static_<version>_amd64.deb`: mutually exclusive packages installing
  the same `/usr/bin/pg_actions` command. Install with `sudo apt install ./<file>`
  so declared dependencies are resolved.
- `ghcr.io/cristicriss17g/pg_actions`: Docker images include PostgreSQL clients;
  choose a `pg16`, `pg17`, or `pg18` tag for the server/tool major you need.
  Unsuffixed current images use PostgreSQL 18 clients.

Static linkage of the application does **not** include the external PostgreSQL
programs. Source/standalone/package users need only the tools their operation
uses:

| Operation | Client programs |
| --- | --- |
| Backup | `pg_dump` |
| Clone | `pg_dump`, `pg_restore` |
| Custom (`bsql`) restore | `pg_restore` |
| SQL restore | `psql` |
| User/database/extension operations | None |

Clients must be major 16 or newer. `pg_dump` must also be at least the source
server's major version. Prefer one coherent suite; clone/check-tools reject mixed
client majors. A newer client's output is not a general downgrade mechanism.
Use your OS's PostgreSQL client package for a compatible major (for example
`postgresql-client-18` from PGDG or `postgresql18-client` on Alpine). A distro's
unversioned package may select an older client, so check it rather than assuming
it is compatible. [PostgreSQL downloads](https://www.postgresql.org/download/).

```bash
pg_actions check-tools
pg_actions --pg-dump /path/to/pg_dump --pg-restore /path/to/pg_restore --psql /path/to/psql check-tools
```

Tool checks show resolved paths/full versions, reject failing version commands,
and time out stalled version checks. Utility commands do not need a database
password. Native Windows release artifacts and managed client downloads remain
future work; Windows users can use an appropriate Docker/WSL environment.

## Connections and credentials

Existing flags/environment variables remain available:

| Flag | Environment | Default/fallback |
| --- | --- | --- |
| `-H, --pg-hostname` | `PG_HOSTNAME` | `localhost` |
| `-U, --pg-superuser` | `PG_SUPERUSER` | Matching passfile user, otherwise `postgres` |
| `-p, --pg-port` | `PG_PORT` | Matching passfile port, otherwise `5432` |
| `-P, --pg-password` | `PG_PASS` | `PGPASSWORD`, then ordered passfile match |
| `--pg-dump`, `--pg-restore`, `--psql` | `PG_DUMP`, `PG_RESTORE`, `PSQL` | PATH |
| `--pg-ssl-mode` | `PG_SSL_MODE` | `prefer` |
| `--pg-ssl-root-cert` | `PG_SSL_ROOT_CERT` | Platform trust roots for Rust; libpq's root policy for clients |

Explicit CLI values override their environment values. Explicit connection
credentials override passfile matches. Passfiles are read in order by
host/port/database/user, with `*` wildcards and escaped `:`/`\`. Comments and
unrelated entries are preserved because the app does not rewrite the input file.
Use `PGPASSFILE` to select an input passfile; otherwise the conventional user
passfile is read. Unix passfiles must be owner-only (normally `chmod 600`).
Malformed entries are ignored with a line-number warning, without logging their
contents. An explicit missing/unreadable passfile is an error.

The CLI opens an administrative connection to `postgres` for database operations;
provide a matching password/entry for it as well as any database-specific entries.
An interactive invocation can prompt for a missing password. Noninteractive
execution does not prompt; passwordless/trust authentication is allowed when the
server supports it. Each child process receives a private temporary passfile and
resolved connection settings, so inherited libpq service/password settings do
not replace the selected credentials. No supplied password is persisted in the
user's passfile.

### TLS

**Version 2 defaults to `prefer`: use TLS when available without certificate or
hostname verification, and connect without TLS when the server does not support
it.** This preserves existing connections to non-TLS and private/self-signed TLS
servers. Database commands warn once per invocation when the mode is implicit:
the planned default for **version 3.0** is `verify-full`. Set `--pg-ssl-mode` or
`PG_SSL_MODE` explicitly to select a policy and suppress that migration warning.

To require trusted TLS and hostname verification now, select `verify-full` and
supply a CA bundle for a private PostgreSQL CA:

```bash
PG_SSL_MODE=verify-full PG_SSL_ROOT_CERT=/path/to/database-ca.pem pg_actions -H db.example.com database list
```

For an intentionally unencrypted local/test server:

```bash
pg_actions --pg-ssl-mode disable database list
```

`--pg-ssl-mode require` requires TLS without the Rust client's
certificate/hostname verification; libpq also validates the CA if a root
certificate is present. `--pg-ssl-mode prefer` explicitly retains the v2 default.
Both Rust connections and child PostgreSQL tools receive the selected policy.
With `verify-full`, the Rust client uses platform roots plus an optional CA
bundle; for a private CA supply that bundle so libpq children use the same root
material.

A configuration-file override is planned, but configuration files are not yet
implemented; current overrides use the CLI or environment.

### S3/RustFS

The existing `S3_ENDPOINT`, `S3_ACCESS_KEY`, `S3_SECRET_KEY`, `S3_BUCKET`,
`S3_REGION`, and `S3_PREFIX` settings have corresponding `--s3-*` flags.
`S3_SESSION_TOKEN`/`--s3-session-token` supports temporary credentials. Endpoint,
keys, bucket, region and a nonempty valid prefix are currently required for
S3 operations. Existing S3 URL/prefix semantics are unchanged; their redesign is
deferred.

Small/empty objects use PUT. Larger objects use sized streaming multipart parts,
with bounded-buffer reads, calculated part sizes, retries for transient failures,
and an abort attempt if uploading/completing fails. Configure an object-store
lifecycle rule for incomplete uploads as well: a killed process or lost create
response cannot always obtain an upload ID to abort.

Config files and OS-backed secret storage are not implemented yet.

## Common operations

```bash
pg_actions database list -o json
pg_actions database create app_copy --owner app_owner
pg_actions database update app_copy --new-owner another_owner
pg_actions database extension app_copy create hstore
pg_actions user create app_owner
pg_actions user grant app_owner --database app_copy --schema public --privileges full
```

Creating a login role without a supplied password generates one and emits a
single JSON object containing username/password **on stdout after success**.
Save that output deliberately. Supplied/generated credentials and restored SQL
are not written to application logs; verbosity does not enable dependency SQL or
signed-request tracing. Optional log files are private on Unix. A database update
requires `--new-owner`; it never silently defaults ownership to the superuser.

```bash
pg_actions clone -d app -n app_copy --new-owner app_owner
pg_actions clone -d app -n app_copy --pg-hostname2 other_host --pg-port2 5433 --pg-superuser2 postgres
```

Destination host/port/user/password overrides are resolved independently, including
another port/user on the same host. Identical source/destination databases are
rejected before overwrite, including the common same-server hostname aliases.
Use `--create-owner` to create a missing requested owner, and `--keep-dump` to
retain the private clone workspace.

### Backup formats and compression

```bash
pg_actions backup app -o app-backup.tar.zst
pg_actions backup all -o all-backup --compression xz --compression-level 1
pg_actions backup app --no-archive --format sql -o app.sql
pg_actions backup app --no-archive --format bsql -o app.bsql
pg_actions backup app                         # upload to configured S3/RustFS
```

Defaults: custom PostgreSQL dump (`bsql`), zstd archive at level 3, one compression
thread and five concurrent database dumps. XZ output/restore remains supported;
its default level is 9. `--compression-threads` is separate from database `--jobs`.
Archived dumps disable PostgreSQL inner compression unless explicitly overridden;
raw dumps and clone retain PostgreSQL's normal compression default. Use
`--pg-dump-compression default|none` to choose explicitly.

`all` always produces an archive, including with `--no-archive`. A missing archive
suffix is added; conflicting codec suffixes are rejected. Bare relative output
paths work. Output is replaced only after successful dump/archive completion.
Temporary files use private per-run OS temp directories. Database names that are
unsafe/nonportable filenames are encoded, with a small `files.json` name mapping
inside new archives; ordinary lowercase names keep their historical filenames.
Use `--keep-temp` to retain a backup's workspace on success or failure.

An `all` backup consists of independent database dumps; it is not a cluster-wide
recovery backup of roles/tablespaces/grants. The tool's restore intentionally
skips original ownership/privileges before applying any requested owner.

### Restore and overwrite safety

```bash
pg_actions restore app_copy -i app-backup.tar.zst --file app --owner app_owner
pg_actions restore app_copy -i app.bsql --overwrite
pg_actions restore app_copy -i app.sql --format sql --overwrite
pg_actions restore app_copy -i s3://configured_bucket/configured_prefix/backup.tar.xz --file app
```

Both archive codecs are detected from their headers, including renamed archives.
Extraction validates the full compression stream before restore. Custom archives
are checked with `pg_restore --list`. Raw SQL restores use `psql -X` and
`ON_ERROR_STOP=1`; custom restores use `--exit-on-error`.

Restore/clone populate a new staging database, apply the requested ownership,
and only then publish the destination name. Overwrite renames the old/staging
DBs in a transaction; failure before commit preserves the original destination.
Old-database cleanup failure retains it under a reported `pg_actions_previous_*`
name without failing successful publication. Other failed staging DBs are cleaned
up, and cleanup errors do not replace the primary operation error.

This requires `CREATEDB`, suitable owner/rename privileges, and space for the
staged copy alongside the existing database. Active connections are terminated
at publication; applications may need to reconnect. `postgres`, `template0` and
`template1` cannot be replaced by staged restore. Restore only trusted dumps:
PostgreSQL/SQL dumps can execute arbitrary code; staging is failure isolation,
not an SQL sandbox. `--keep-temp` retains downloaded/extracted files and prints
the actual directory. The old restore `--jobs` setting still has no effect;
parallel restore remains deferred.

Ownership updates cover ordinary application schemas, tables/partitions, views,
materialized views, sequences, enum/base/composite/range/domain types,
functions/procedures/aggregates, and large objects in a transaction. System
schemas and extension member objects retain their ownership.

## Output, logging and help

List commands support `json`, `json-compact`, `json-lines`, `csv`, `table` and
`simple`. Existing JSON entity shapes are preserved. Empty JSON lists emit `[]`;
CSV emits a header even for empty results. Fuzzy matches rank by decreasing
relevance. Logs go to stderr; `-v`/`-vv` affect app logs and
`--use-json-logging` selects structured logs.

See [the generated CLI reference](docs/cli.md) for current flags and aliases.
Generate shell completions without database credentials:

```bash
pg_actions completions bash
pg_actions completions powershell
```

## Development and validation

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo build --locked
python3 tests/maintenance.py --docker
```

The maintenance suite creates/removes its own PostgreSQL container, local CA and
mock S3 service; it does not use existing databases or passfiles. It requires
Docker, PostgreSQL 18 clients (matching the fixture) and OpenSSL. GitHub CI
installs `postgresql-client-18` from PGDG and selects its binary directory
explicitly. Tests cover credential logging/precedence,
quoted SQL names, staging safety, ownership, TLS and S3 upload failures.

The existing [archive/Compose suite](tests/archive-compose.py) remains available
for data/codec checks and synthetic benchmarks. It requires an explicitly running
Compose DB plus PostgreSQL clients, tar/xz/zstd; it creates/removes its own fixture
DBs. Historical `tests/tests.sh` commands are manual examples, not a test suite.

GitHub publication waits for format, strict Clippy, unit and maintenance tests.
Autofix applies rustfmt and available Clippy fixes, tests them, and commits/pushes
changed Rust files to the original branch. It then dispatches CI checks for the
updated commit without packaging/publishing again. Fork and Dependabot PRs are
excluded because their workflow tokens cannot write back to the branch.

See [review and implementation status](docs/review/README.md).

## License

GNU Affero General Public License v3.0 only. See [LICENSE](LICENSE).
