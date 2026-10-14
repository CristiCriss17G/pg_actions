# Configuration and credential storage feasibility

**Snapshot note:** this investigation predates the selected maintenance changes. Shared credential/tool/path foundations now exist; see [implementation status](06-maintenance-implementation.md). The exploratory feature itself remains deferred.

Investigated on 2026-10-14. **Proposal only; no configuration feature or dependency was added.** “S2 credentials” is interpreted as S3/object-store credentials, matching this application's S3/RustFS support.

## Finding and recommendation

**Feasible, with moderate application changes.** Add a versioned TOML file with named connection profiles and command defaults. Store ordinary settings in it; store secret references there and resolve passwords/keys from environment variables, private secret files, an existing PostgreSQL passfile, or an OS credential store. Support private plaintext secrets as a clearly identified opt-in convenience, rather than calling a permission-restricted file encrypted storage.

The application already has Serde, Clap environment support and `dirs`; it lacks file loading, profile selection and a coherent credential resolver. `main.rs:171–210` currently lets a host-matching passfile override explicit credentials, and `db/general.rs:36` rewrites that file. Fix that foundation as part of implementation, not by layering more precedence on top of it. See maintenance A01–A03/A14/A17/A20.

## Proposed file and discovery contract

1. `--no-config` disables file loading.
2. Otherwise `--config FILE` overrides `PG_ACTIONS_CONFIG`.
3. Otherwise read `dirs::config_dir()/pg_actions/config.toml` if present.
4. A missing implicit file is fine; a missing explicit file or malformed present file is an error. Help/version/completions should remain usable without credentials or config.

Use the already installed `dirs` package. Its config directory follows XDG on Linux, Application Support on macOS and roaming application data on Windows. Proposed resulting paths are `~/.config/pg_actions/config.toml` (subject to XDG overrides), `~/Library/Application Support/pg_actions/config.toml`, and `%APPDATA%\pg_actions\config.toml`. This also matches the inspected local `dirs 7.0.0` source. [Upstream config directory documentation](https://docs.rs/dirs/latest/dirs/fn.config_dir.html).

Do not automatically search the working directory or parent directories for credential config. Support project config through an explicit path. Resolve relative certificate/secret-file/tool paths relative to the config file; keep CLI-provided paths relative to the invocation directory. Define `${...}` substitution only if explicitly supported; TOML itself should not execute commands or expand arbitrary shell expressions.

## Proposed schema

This is an illustrative future schema, not a file the current binary accepts. Hostnames, account names and environment-variable names below are placeholders.

```toml
config_version = 1
default_profile = "development"

[defaults.backup]
jobs = 2
format = "bsql"
compression = "zstd"
compression_level = 3
compression_threads = 1
# Leave pg_dump_compression unset to preserve today's archive/raw behavior.

[defaults.logging]
json = false

[tools]
source = "system"
auto_download = false

[profiles.development.postgres]
hostname = "db.example.invalid"
port = 5432
username = "backup_user"
password = { provider = "keyring", name = "development/postgres/password" }
tls_mode = "verify-full"
ca_file = "certs/database-ca.pem"

[profiles.development.s3]
endpoint = "https://objects.example.invalid"
region = "example-region"
bucket = "example-backups"
prefix = "development"
access_key = { provider = "env", name = "S3_ACCESS_KEY" }
secret_key = { provider = "file", path = "secrets/s3-secret-key" }

[profiles.destination.postgres]
hostname = "destination.example.invalid"
port = 5432
username = "restore_user"
password = { provider = "env", name = "DESTINATION_PG_PASSWORD" }
```

Only default non-destructive preferences: jobs, codecs/levels, format, output/logging preferences, tool source and connection settings. Keep destination DB names, overwrite/drop decisions and privilege changes explicit per invocation. If persistent destructive defaults are ever offered, that should be a separate product decision with visible effective behavior, not an accidental result of loading TOML.

## Precedence and parsing

Recommended precedence, highest first: **explicit CLI value → existing environment value → selected profile override → file-wide default → built-in default**. Apply it field by field, not by replacing an entire connection whenever one field changes. Proposed `--profile NAME`/`PG_ACTIONS_PROFILE` selects the source; `--destination-profile NAME` selects clone's destination. Existing `PG_HOSTNAME2`/port/user/password overrides affect only that destination. Missing profile is an error, not a silent fallthrough to localhost.

Clap's `ArgMatches::value_source` distinguishes command-line, environment and default values; `contains_id` alone does not. Use `Cli::command().get_matches()` with `FromArgMatches`, retain the matches, and merge using origin information, including subcommand/global arguments. This prevents existing `default_value` fields such as port/jobs from silently overwriting file settings. [Clap value-source documentation](https://docs.rs/clap/latest/clap/struct.ArgMatches.html#method.value_source).

Boolean settings need an explicit false override: add a deliberate `--flag=false` or matching negative-option contract where appropriate, and do not merge parser-default false as if the user supplied it. Preserve existing flags/aliases and test global options before/after subcommands. Keep source metadata so a redacted `config show --effective` can explain where each setting came from.

Use separate optional `FileConfig` structs and validated `ResolvedConfig`/`ResolvedConnection` types. Add `toml` and deserialize through existing Serde, with schema versioning and unknown-key errors. TOML supports typed Serde deserialization directly. [TOML crate documentation](https://docs.rs/toml/latest/toml/).

For this scope, an explicit small merge is easier to review than generic multi-format layering. `config` and `figment` are feasible alternatives if many sources/formats become requirements; neither resolves this app's credential precedence automatically. [config providers](https://docs.rs/config/latest/config/), [Figment providers and merging](https://docs.rs/figment/latest/figment/).

## Secret storage choices

| Method | Suitable use | Limits and implementation |
| --- | --- | --- |
| Environment reference | CI and existing integrations | Easy migration; avoid logging/dumping environments or placing literal secrets in shell examples |
| Private file reference | Containers, cron, mounted secret stores | Read on demand; enforce ownership/permissions as appropriate; define single trailing-newline handling |
| Existing passfile | Compatibility with PostgreSQL users | Implement ordered connection matching and escaping; do not rewrite it as a side effect |
| OS credential store | Interactive Windows/macOS/Linux desktop use | Store profile-scoped entries; handle locked/unavailable stores and service-account context explicitly |
| Private plaintext config | Explicit local convenience | Unix owner-only directory/file policy; Windows restricted ACLs; not encryption at rest |
| Encrypted config/secret file | Portable encrypted export or offline use | Needs a separate key/passphrase lifecycle; defer until there is a clear requirement |

For passfile compatibility, libpq supports `PGPASSFILE`, escaped separators and ordered first-match selection; Windows uses `%APPDATA%\postgresql\pgpass.conf`. These requirements should define the replacement for the current host-only parser. [PostgreSQL passfile specification](https://www.postgresql.org/docs/current/libpq-pgpass.html).

OS-backed storage is possible. Current `keyring` documentation distinguishes simple convenience APIs from explicitly selected stores; for controlled backends, evaluate `keyring-core` plus platform stores rather than copying older keyring v3 feature recipes. Windows has a native credential-store implementation. Choose matching crate versions/features after checking the project's Rust 1.96 baseline. [Current keyring guidance](https://docs.rs/keyring/latest/keyring/), [keyring-core](https://docs.rs/keyring-core/latest/keyring_core/), [Windows store](https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/).

Linux headless/container/cron environments must not depend on an unlocked desktop Secret Service. Keep environment/private-file providers available. Failure to resolve an explicitly configured secret is an error, not silent fallback to another account. Retrieve only secrets needed by the selected command. An OS store protects persisted secrets according to its backend; it does not protect them from the same running user's processes after retrieval.

Encryption is feasible but is a separate feature: use an established authenticated encryption format/library, keep the key outside the encrypted file, support an interactive passphrase or external key provider, and document recovery/rotation. Storing an encryption key next to ciphertext gives little benefit. No custom cryptography is needed for the initial design.

## Integration method

1. Remove credential logging and stop permanent passfile rewriting (A01/A02). Add redacted secret types and typed secret references.
2. Add `utils/config.rs` for file structs, discovery, schema validation and source-aware merge; add CLI config/profile options in `main.rs`.
3. Add `utils/credentials.rs` for provider resolution and invocation-owned credentials; refactor connection/tool consumers away from hostname maps. Resolve TLS policy consistently for Rust connections and libpq children.
4. Dispatch utility/config/tool commands before password resolution. Suggested future commands: `config init`, `config validate`, `config show --effective` (always redacted), and `credentials set PROFILE FIELD` with an interactive prompt.
5. Give child tools a private temporary `PGPASSFILE` owned for the duration of the operation. Do not include resolved passwords on their command lines. Add keyring storage as a later backend without changing file schema.

Config-only work is medium in scope because precedence touches nearly every command; adding OS stores introduces platform/service-context testing. Encrypted export is additional scope and should follow the ordinary/provider-based config feature.

## Acceptance criteria

Test CLI/env/profile/default precedence for every supported scalar and boolean, explicit false, destination isolation, unknown keys/schema versions, invalid ports/jobs/codec combinations, explicit missing files, relative paths and unavailable providers. Verify utility commands with no secrets and ensure help/errors/debug/JSON/effective-config output never contains sentinel secrets. Exercise provider-backed connection and S3 operations with disposable fixtures. Test Unix file privacy and Windows ACLs independently. No new feature or provider was built or exercised during this investigation.
