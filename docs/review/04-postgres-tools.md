# PostgreSQL tools, bundling and installation feasibility

**Snapshot note:** this investigation predates the selected maintenance changes. Shared credential/tool/path foundations now exist; see [implementation status](06-maintenance-implementation.md). The exploratory feature itself remains deferred.

Investigated on 2026-10-14. **Proposal only: no tools were installed, downloaded, bundled or published.**

## Finding and recommendation

**All three approaches are feasible:** improve ordinary installation, ship companion client tools, or download a verified client suite at runtime. The most practical sequence is better discovery/checks first, optional portable bundles second, and a managed per-user download/cache third. Keep the existing small binary and system-tool path as supported options.

Do not equate a statically linked Rust executable with bundled PostgreSQL programs. They are separate child processes, with their own shared-library and version requirements. Embedding executable bytes in the Rust binary is possible but still requires extraction to disk and platform-specific packaging; a portable directory/ZIP is simpler and easier to inspect/update.

## Existing integration points

`main.rs:71–86` already supports explicit `PG_DUMP`, `PG_RESTORE` and `PSQL` paths. `PGTools` holds them; `db/general.rs` invokes them through `tokio::process::Command`, not shell strings. `misc.rs:456–511` checks versions using a string-keyed map, only enforces >=16 and ignores the version process's exit code.

Docker already installs PostgreSQL client packages: `Dockerfile` defaults to 18, and `.github/workflows/docker-build.yml` builds 16/17/18 variants. Standalone CI installs client 16 in the build environment but uploads only the Rust binary: the clients are not transferred to the release. Debian control files do not install/recommend them. No current resolver discovers versioned installations or validates compatibility with a server/archive.

This work should incorporate maintenance A09/A14/A15/A17/A19; C1 is unrelated except that future S3/error refactoring must preserve conversions.

## What each command actually needs

| Operation | Required executables |
| --- | --- |
| Backup, SQL or custom format | `pg_dump` |
| Clone | `pg_dump` and `pg_restore` |
| Restore custom `.bsql` | `pg_restore` |
| Restore SQL | `psql` |
| User/database/extension administration | None; these use the Rust PostgreSQL client |
| Optional future cluster-globals backup | `pg_dumpall` |

Make `check-tools` inspect the full requested suite, while ordinary commands require only their actual capabilities. Missing `psql` should not block a custom backup. This change alone improves installation without shipping new assets.

## Version policy

PostgreSQL documents that `pg_dump` cannot dump a server newer than its own major version; loading a newer client's output into an older server is not guaranteed. Therefore a minimum of 16 does not establish compatibility. [pg_dump compatibility rules](https://www.postgresql.org/docs/current/app-pgdump.html).

Recommended project policy: prefer a suite matching the source server's major version for backup/clone, and a compatible restore suite tied to the dump's producing version and tested target major. Discover `server_version_num` through the already-used Rust connection before choosing a suite; inspect custom dumps with `pg_restore --list` and preserve producing-tool/server versions in a backup manifest. For old dumps without a manifest, allow an explicit override and check archive support before deleting a destination. Do not guarantee downgrades merely because a client executable can read an archive.

Use one coherent suite rather than accidentally mixing `pg_dump 18` from one install with `pg_restore 16` from another. Preserve explicit per-tool overrides but diagnose mixed versions. Record exact paths and full versions in `check-tools`/redacted logs. Pin exact patch versions/digests for managed assets; explicit updates change that pin. Start the test/bundle matrix with the existing 16/17/18 project majors, rather than automatically including prereleases.

## Option comparison

| Approach | Feasibility | User experience | Project cost |
| --- | --- | --- | --- |
| Better diagnostics/package integration | High | One clear package install or automatic discovery | Low to medium |
| Current Docker distributions | Already configured | Client tools included; requires Docker and suitable mounts/networking | Existing maintenance |
| Portable app + client-suite archive | High after per-platform builds | Download/unpack and run; works offline | Medium to high release/ABI work |
| Verified per-user runtime download | High after portable suites exist | Small initial install, managed setup without admin | Medium to high resolver/cache/security work |
| Executables embedded in one Rust binary | Possible | One initial file, but still extracts external tools | Higher size/complexity with little benefit over an archive |
| Replace pg_dump/pg_restore in Rust | Technically possible but poor fit | No external clients | Very high ongoing format/version fidelity burden |

The Rust protocol client can administer PostgreSQL but is not a replacement for its dump/restore formats, catalog traversal and version handling. Retaining upstream tools avoids a substantial new backup implementation.

## A shared resolver for every distribution

Introduce `utils/pg_tools.rs` with `ToolRequirement`, `ToolSuite { directory, major, version, origin, paths }`, and validation. Preserve existing explicit paths; add proposed `--pg-tools-dir`/`PG_TOOLS_DIR` plus config settings from [03-configuration.md](03-configuration.md).

Default resolution order should be documented: explicit per-tool paths → explicitly selected suite directory → shipped suite beside the executable → matching verified managed cache → discovered system installation/PATH. An explicit `source = "system"` or `source = "managed"` should constrain that search; an invalid explicit path should fail, not silently run another executable. Check executable exit status, architecture and required version. Resolve absolute paths once and use those throughout the invocation.

On Linux, inspect configured versioned PostgreSQL directories as well as PATH; on Windows, inspect explicit/configured directories and conventional installed PostgreSQL locations. Directory existence is not sufficient: verify the binaries. If no suite meets requirements, show the missing tool, detected version/server mismatch and actionable installation choices.

## Portable client suites

Proposed layout:

```text
pg_actions[.exe]
postgres/<major>/bin/pg_dump[.exe]
postgres/<major>/bin/pg_restore[.exe]
postgres/<major>/bin/psql[.exe]
postgres/<major>/lib/...             # where the platform needs it
postgres/<major>/share/...           # runtime resources when needed
licenses/...
manifest.json
```

Package the dependencies actually needed by the shipped build: libpq, TLS/crypto, compression libraries and platform runtimes as applicable. Inventory them with Linux ELF tools, macOS dependency tools if that platform is later added, and Windows PE dependency checks. A Linux musl bundle and glibc bundle are different targets; copying Alpine clients into a generic glibc release does not establish portability. Verify relocated archives on a clean machine/container without PostgreSQL installed or build-host library paths.

For Linux, building pinned upstream source is feasible. PostgreSQL documents a client-only **installation** using selected `make -C ... install` targets; this is not a universal single client-only configure switch. Build/stage a pinned source release, retain just the required executables and their dependency closure, then test relocation. Meson also supports Windows builds and documents relocatable-layout/rpath considerations. [Make installation targets](https://www.postgresql.org/docs/current/install-make.html), [Meson build/layout documentation](https://www.postgresql.org/docs/current/install-meson.html).

For Windows, the PostgreSQL project links an EDB-hosted installer and binary ZIP distribution. That makes a binary ZIP a candidate upstream source; it is not a promise of a minimal client-only package, stable unattended download API or reviewed redistribution of every included component. Either select/test an approved EDB archive or build a project-owned client suite from pinned source. The EDB page did not expose a complete version/checksum catalog in this research, so specific asset URLs/digests remain implementation work. [Official Windows downloads](https://www.postgresql.org/download/windows/).

PostgreSQL's license permits redistribution subject to retaining its notice. Inventory the actual bundled libraries and their notices separately before distributing suites; the application's AGPL license does not eliminate those requirements. [PostgreSQL license](https://www.postgresql.org/about/licence/).

## Managed download method

Offer an explicit future `pg_actions tools install --major 18` (syntax proposed), then reuse its installer for an opt-in `auto_download` setting. Default to no silent network download during a destructive operation. Tool setup/check commands must not prompt for a database password (A14).

1. Identify OS, architecture and Linux ABI; map to a tested asset, not a guessed filename. Reject unsupported targets clearly.
2. Select a pinned release manifest listing URL, length, full version, target, SHA-256 and required contents. Anchor trust in a release signature/embedded trusted digest policy, not a mutable “latest” URL or a checksum downloaded from an equally untrusted mirror.
3. Download over verified HTTPS with bounded retries/timeouts into a private staging directory. Enforce compressed and unpacked size limits.
4. Verify the archive before extraction. Reject absolute/traversing paths, unsafe links and unexpected content; use a dedicated safe extraction policy rather than blindly trusting the downloaded tar/ZIP.
5. Check the installed executables and their dependency resolution from the staged path. Lock per asset/version and atomically publish a completed suite; interrupted installs never become selectable.
6. Cache suites under `dirs::cache_dir()/pg_actions/tools/<target>/<full-version>` and use absolute executable paths. Config can override the cache root for controlled/immutable environments. The local `dirs` dependency already implements OS-specific cache locations. [Upstream cache directory documentation](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html).
7. Support offline use, explicit update/removal, an explicit verified archive import for air-gapped use, and a configured trusted mirror. Do not invoke sudo or system package managers implicitly. Keep old suites available for reproducible restores until deliberately removed.

Add an explicit HTTP client, checksum verification, locking and ZIP extraction support as needed; do not route tool distribution through database/S3 secrets. Release CI publishes immutable portable assets plus checksums/provenance only after tests pass (A18). Capture an inventory of shipped components for each suite and patch them when upstream fixes land. For Windows, keep DLLs in private known locations and avoid depending on untrusted working-directory DLL search paths.

## Improve installation before managed downloads

- Make README distinguish “Rust binary runtime” from “client tools required for the selected operation.” Remove blanket install/upgrade sequences; explain how to select a compatible major.
- For Debian packages, deliberately choose required client dependencies for a full-feature package or `Recommends`/an optional tools package for a minimal administrative install. A versioned `postgresql-client-N` is predictable but needs repository availability; a generic metapackage may select an incompatible major. Generate actual Rust-binary library dependencies and test the stated supported distributions.
- Ship compressed portable release assets with executable bits/layout preserved, checksums and a short install guide. Name platform/ABI/architecture explicitly; do not call a bare Rust binary an all-tools static bundle.
- Keep Docker images as the simplest existing fully provisioned automated path; show version-specific tags and safe credential/file mounts. Windows Docker/WSL is an alternative workflow, not native Windows support.
- Add package-manager distribution only after artifacts/contracts stabilize; Windows ZIP first, then evaluate a manifest for winget/Scoop. Package submissions, signing and publication are follow-up work, not changes made in this investigation.

## Implementation order and acceptance

Phase 1 (small/medium): shared resolver, command-specific requirements, status-aware checks, server/version diagnostics and clearer installation docs. Phase 2 (medium/high): Linux and Windows portable suites and clean-host dependency/relocation tests. Phase 3 (medium/high): signed/pinned downloader, cache, offline mode and concurrent-install handling. Optional embedding comes last if a strong single-file requirement remains.

Test missing/non-executable tools, version-command failure (the current probe exposes this bug), mixed suites, a too-old dump client, incompatible archive, explicit-path precedence and paths with spaces. Test a fresh machine with no PostgreSQL install, corrupt downloads, interruption/concurrency, untrusted extraction entries and offline mode. Run dump/restore round trips across the supported source/target major matrix. No portable bundle or downloader was prototyped here, so binary size, startup download time and ABI coverage are intentionally unmeasured.
