# Changelog

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
