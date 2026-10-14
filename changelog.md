# Changelog

## Release 2.0.0 (Unreleased)

Prepared on `dev` for validation before merging into `main`. The merge into
`main` is intended to publish the `v2.0.0` release; the release date remains
unset until that happens.

### Features

- Default new backup archives to zstd (`.tar.zst`), retaining explicit xz output and automatic restoration of both codecs.
- Add codec-specific compression levels, separate compression thread control, and PostgreSQL dump compression selection.
- Add PostgreSQL TLS policy and CA-bundle flags (`--pg-ssl-mode`, `--pg-ssl-root-cert`) with matching environment variables.
- Keep the v2 default as `prefer`: use TLS without certificate/hostname verification when supported, with fallback for non-TLS servers. An implicit-mode warning announces the planned `verify-full` default in v3.0; an explicit CLI/environment mode suppresses the warning.
- Add S3 session-token support and streaming multipart uploads with bounded retries and abort on failure; small/empty uploads use a single PUT.
- Add private per-run workspaces, safe database dump filenames, and `backup --keep-temp` for inspecting retained temporary files.

### Bug fixes and improvements

- Restore/clone into a staging database before publishing it. Overwrites atomically rename the old/staging databases; failed restores preserve the original destination. Report a recovery name if old-database cleanup fails after publication.
- Validate destination names, owners, privileges, custom archives and overwrite requirements before destructive work; reject self-clones and maintenance/template database replacement.
- Stop SQL restores on the first error and propagate `psql`/`pg_restore` failures.
- Reassign application object/database ownership sequentially in a transaction, covering application schemas, relations, types, routines and large objects while excluding system schemas and extension members.
- Quote SQL identifiers and escape password literals consistently, including unusual database/schema/role names; prevent connection-like database names from becoming libpq connection overrides.
- Read passfiles without rewriting them, respecting entry order, wildcards, escapes and connection matching. Resolve source/destination credentials separately and give child tools private temporary passfiles.
- Remove passwords and restored SQL/data from logs, redact credential debug output, and filter dependency trace logging. Emit generated login passwords once as JSON on stdout after successful creation.
- Publish completed dumps, archives and downloads atomically; finish/validate compression streams explicitly and preserve primary errors when cleanup also fails.
- Disable inner pg_dump compression for archived backups by default; preserve PostgreSQL defaults for raw backups and cloning.
- Resolve archive paths consistently, fix `all --no-archive` archive naming, and check complete archive integrity before replacing a destination database.
- Check only the PostgreSQL tools needed by each command, validate tool exit status/version with a timeout, reject mixed required-client majors, and check dump-client/server compatibility. Help, completions and tool checks do not prompt for database credentials.
- Remove avoidable panic paths and duplicated connection/output handling. Box large SDK errors to eliminate Clippy warnings while retaining their error sources.
- Return valid empty JSON/CSV results and rank fuzzy database/user matches by descending relevance.

### Build, packaging and documentation

- Pin the local, CI, Docker and devcontainer Rust toolchain to 1.99.0; require Rust 1.99 for source builds.
- Gate GitHub publication builds/releases on checks. Restore automatic rustfmt/Clippy fixes, tested commits to the source branch, and CI checks for the updated commit.
- Build dynamic GitHub binaries on Ubuntu 22.04 for a stable library baseline; generate Debian shared-library dependencies from the actual ELF binary.
- Install Debian binaries under `/usr/bin` and make dynamic/static packages mutually exclusive. Verify static artifacts have no dynamic interpreter.
- Remove example credentials from distributable Docker defaults and pass the current version to image labels.
- Refresh the README and generated CLI reference. Add maintenance regression tests for credentials, SQL quoting, staging, ownership, TLS and S3 failures, plus archive round-trip/compression comparison tests.

### Upgrade notes

- Clone/restore staging requires `CREATEDB`, suitable ownership/rename privileges and enough disk space for a second database. Publication disconnects active sessions; the maintenance/template databases cannot be staging destinations.
- External PostgreSQL tools must be version 16 or newer and compatible with the server/archive. The Docker image bundles the clients; standalone binaries still require their installation.
- New archives use `.tar.zst` unless xz is selected explicitly. Existing xz archives remain readable; restore detects the codec from the file contents.
- Scripts should consume generated passwords from stdout rather than logs, and database ownership updates require an explicit owner.
- PostgreSQL TLS verification is opt-in in v2. Select `PG_SSL_MODE=verify-full` to enable it now, or explicitly set `prefer`, `require` or `disable` for the desired policy. Configuration-file overrides, managed client downloads and native Windows support remain future work.

## Release 1.5.0 (2025-03-29)

### Miscellaneous

- Updated devcontainer specs for dockerfile apt installs, certificates for minio and installed vs code extensions.
- Updated `.gitlab-ci.yml` to properly take the MR description for the release and install the proper postgresql packages.

### Bug fixes and improvements

- [Improve] Changes use of `&str` to `AsRef<Path>` for explicit errors on paths and more efficient memory usage.
- [Improve] The `grant` and `revoke` commands, at the `full` level, now include altering the default privileges for a user, to propagate to newly created objects.
- [Fix] Fixed behaviour for the `revoke` to not exit on first error, but collect and report all errors at the end, the same as the `grant` command.
- [Fix] Update use of `rand::distributions` to `rand::distr` and `rand::thread_rng` to `rand::rng` to avoid deprecation warnings.

### Features

- [Add] Added the `restore` command to restore a database from a backup file. This command uses the `pg_restore` utility to restore the database from a backup file. More details about usage can be found in the [`README.md`](README.md) file.
- [Add] Added argument for `--psql` to specify a custom psql command to use for the `psql` command. This allows the user to specify a custom psql command to use for the `psql` command.
