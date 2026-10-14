# Generated CLI reference

Generated from the Rust 1.99 maintenance checkout; regenerate after CLI changes.

## pg_actions

```text
A simple CLI tool for managing Postgres databases

Usage: pg_actions [OPTIONS] <COMMAND>

Commands:
  clone        Clone a database [alias: cl]
  backup       Backup operations
  restore      Restore operations
  user         User operations
  database     Database operations
  completions  Generate shell completions
  check-tools  Do a basic check of the tools This is useful for CI/CD pipelines [alias: ct]
  help         Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## clone

```text
Clone a database

Usage: pg_actions clone [OPTIONS] --database <DATABASE> --new-database <NEW_DATABASE>

Options:
  -d, --database <DATABASE>
          database to clone

  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -n, --new-database <NEW_DATABASE>
          new destination database

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

      --overwrite
          overwrite new database if it exists

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -o, --new-owner <NEW_OWNER>
          owner user for db

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

  -c, --create-owner
          create the new owner user

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

  -s, --new-password <NEW_PASSWORD>
          new password for db provide if the user does not exist

  -k, --keep-dump
          keep the dump file defaults to false
          
          [env: KEEP_DUMP=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-hostname2 <PG_HOSTNAME2>
          Optional new host
          
          [env: PG_HOSTNAME2=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-port2 <PG_PORT2>
          Optional new port
          
          [env: PG_PORT2=]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --pg-superuser2 <PG_SUPERUSER2>
          Optional user for new host
          
          [env: PG_SUPERUSER2=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --pg-password2 <PG_PASSWORD2>
          Optional password for new host
          
          [env: PG_PASSWORD2]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## backup

```text
Backup operations

Usage: pg_actions backup [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>
          database to backup defaults to all

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -o, --output-location <OUTPUT_LOCATION>
          Output file path; defaults to S3 with a timestamped name. Archive suffixes are .tar.zst or
          .tar.xz according to --compression
          
          [env: BACKUP_LOCATION=]

  -j, --jobs <JOBS>
          Concurrent database dumps (independent of compression threads)
          
          [env: JOBS=]
          [default: 5]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -f, --format <FORMAT>
          Format of the output file defaults to bsql
          
          [env: FORMAT=]
          [default: bsql]
          [possible values: sql, bsql]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

      --no-archive
          Create archive or just files, has no effect on `all` backups
          
          [env: NO_ARCHIVE=]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --keep-temp
          Keep private staging files on success or failure
          
          [env: KEEP_TEMP=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --compression <COMPRESSION>
          Archive compression codec
          
          [env: COMPRESSION=]
          [default: zstd]
          [possible values: zstd, xz]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --compression-level <COMPRESSION_LEVEL>
          Archive compression level: zstd 1-22 (default 3), xz 0-9 (default 9)
          
          [env: COMPRESSION_LEVEL=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --compression-threads <COMPRESSION_THREADS>
          Compression worker threads; 1 disables parallel compression. Parallel xz at high levels
          can require substantial memory
          
          [env: COMPRESSION_THREADS=]
          [default: 1]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-dump-compression <PG_DUMP_COMPRESSION>
          pg_dump compression: defaults to none for archives, default for raw files

          Possible values:
          - default: Let pg_dump use its normal compression settings
          - none:    Disable pg_dump compression; the outer archive handles compression
          
          [env: PG_DUMP_COMPRESSION=]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## restore

```text
Restore operations

Usage: pg_actions restore [OPTIONS] --input-location <INPUT_LOCATION> <DATABASE>

Arguments:
  <DATABASE>
          database to restore

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -i, --input-location <INPUT_LOCATION>
          input location chose between s3 or a file path when s3 is chosen, the input location
          should be in the format s3://bucket_name/file_path you can also specify all the S3_*
          args/environment variables to use S3, the file url will have priority
          
          [env: RESTORE_LOCATION=]

  -j, --jobs <JOBS>
          Parallel jobs to use defaults to 5
          
          [env: JOBS=]
          [default: 5]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -f, --format <FORMAT>
          Format of the input file defaults to bsql
          
          [env: FORMAT=]
          [default: bsql]
          [possible values: sql, bsql]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

      --file <FILE_NAME>
          File name, without extension, in case of archive extraction and the file name is different
          from the database name
          
          [env: FILE_NAME=]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

  -o, --owner <OWNER>
          Owner user for db

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --overwrite
          overwrite the database if it exists
          
          [env: OVERWRITE=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

  -k, --keep-temp
          keep the temp files defaults to false
          
          [env: KEEP_TEMP=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user

```text
User operations

Usage: pg_actions user [OPTIONS] <COMMAND>

Commands:
  list    List users
  create  Create a user
  delete  Delete a user
  update  Update a user
  grant   Grant privileges to a user
  revoke  Revoke privileges from a user
  help    Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user list

```text
List users

Usage: pg_actions user list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

      --sort <SORT>
          sort by username
          
          [possible values: asc, desc]

  -o, --output <OUTPUT>
          quiet mode
          
          [default: table]
          [possible values: json, json-compact, json-lines, csv, table, simple]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -e, --extra
          extra details

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

  -q, --query <QUERY>
          query string, fuzzy search

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user create

```text
Create a user

Usage: pg_actions user create [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>
          username

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -s, --password <PASSWORD>
          password

      --superuser
          user as superuser

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

      --createdb
          createdb for user

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -n, --no-login
          role with no login defaults to false

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user update

```text
Update a user

Usage: pg_actions user update [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>
          username

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -s, --password <PASSWORD>
          password for user

      --superuser <SUPERUSER>
          user as superuser
          
          [possible values: true, false]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

      --createdb <CREATEDB>
          createdb for user
          
          [possible values: true, false]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -n, --no-login <NO_LOGIN>
          role with no login
          
          [possible values: true, false]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user grant

```text
Grant privileges to a user

Usage: pg_actions user grant [OPTIONS] --database <DATABASE> --privileges <PRIVILEGES> <USERNAME>

Arguments:
  <USERNAME>
          username

Options:
  -d, --database <DATABASE>
          database

  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

      --schema <SCHEMA>
          schema
          
          [default: public]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -g, --privileges <PRIVILEGES>
          privileges full or read-only or read-update-only
          
          [possible values: full, read-only, read-update-only]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## user revoke

```text
Revoke privileges from a user

Usage: pg_actions user revoke [OPTIONS] --database <DATABASE> --privileges <PRIVILEGES> <USERNAME>

Arguments:
  <USERNAME>
          username

Options:
  -d, --database <DATABASE>
          database

  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

      --schema <SCHEMA>
          schema
          
          [default: public]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -g, --privileges <PRIVILEGES>
          privileges
          
          [possible values: full, read-only, read-update-only]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database

```text
Database operations

Usage: pg_actions database [OPTIONS] <COMMAND>

Commands:
  list       List databases
  create     Create a database
  delete     Delete a database
  update     Update a database
  extension  Manipulate extensions on a database [alias: ext]
  help       Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database list

```text
List databases

Usage: pg_actions database list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

      --sort <SORT>
          sort by database name
          
          [possible values: asc, desc]

  -o, --output <OUTPUT>
          quiet mode
          
          [default: table]
          [possible values: json, json-compact, json-lines, csv, table, simple]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -e, --extra
          extra details

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

  -q, --query <QUERY>
          query string, fuzzy search

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database create

```text
Create a database

Usage: pg_actions database create [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>
          database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -o, --owner <OWNER>
          owner user for db

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database update

```text
Update a database

Usage: pg_actions database update [OPTIONS] --new-owner <NEW_OWNER> <DATABASE>

Arguments:
  <DATABASE>
          database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -o, --new-owner <NEW_OWNER>
          new owner user for db

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database extension

```text
Manipulate extensions on a database

Usage: pg_actions database extension [OPTIONS] <DATABASE> <COMMAND>

Commands:
  list    List extensions
  create  Create an extension
  delete  Delete an extension
  help    Print this message or the help of the given subcommand(s)

Arguments:
  <DATABASE>
          Database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## database extension list

```text
List extensions

Usage: pg_actions database extension <DATABASE> list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

      --sort <SORT>
          sort by extension name
          
          [possible values: asc, desc]

  -o, --output <OUTPUT>
          quiet mode
          
          [default: table]
          [possible values: json, json-compact, json-lines, csv, table, simple]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -e, --extra
          extra details

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## completions

```text
Generate shell completions

Usage: pg_actions completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>
          The shell to generate the script for
          
          [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```

## check-tools

```text
Do a basic check of the tools This is useful for CI/CD pipelines

Usage: pg_actions check-tools [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL
          
          [env: PG_HOSTNAME=]
          [default: localhost]

  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username
          
          [env: PG_SUPERUSER=]

  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password
          
          [env: PG_PASS]

  -p, --pg-port <PG_PORT>
          Sets Postgres port
          
          [env: PG_PORT=]

      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used
          
          [env: PG_DUMP=]

      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used
          
          [env: PG_RESTORE=]

      --psql <PSQL>
          Optional custom psql path If not set, the default from PATH will be used
          
          [env: PSQL=]

      --pg-ssl-mode <PG_SSL_MODE>
          PostgreSQL TLS policy; defaults to unverified opportunistic TLS in v2, verify-full in v3

          Possible values:
          - disable:     No TLS; intended for a trusted local/test connection
          - prefer:      Use TLS without certificate/hostname verification when available; otherwise
            connect without TLS
          - require:     Require encryption without verifying the server certificate/hostname
          - verify-full: Require encryption and verify certificate trust and hostname
          
          [env: PG_SSL_MODE=]
          [default: prefer]

      --pg-ssl-root-cert <PG_SSL_ROOT_CERT>
          PEM CA certificate bundle for PostgreSQL TLS
          
          [env: PG_SSL_ROOT_CERT=]

      --s3-session-token <S3_SESSION_TOKEN>
          Optional S3 session token for temporary credentials
          
          [env: S3_SESSION_TOKEN]

      --s3-endpoint <S3_ENDPOINT>
          S3/RustFS endpoint
          
          [env: S3_ENDPOINT=]

      --s3-access-key <S3_ACCESS_KEY>
          S3/RustFS access key
          
          [env: S3_ACCESS_KEY]

      --s3-secret-key <S3_SECRET_KEY>
          S3/RustFS secret key
          
          [env: S3_SECRET_KEY]

      --s3-bucket <S3_BUCKET>
          S3/RustFS bucket
          
          [env: S3_BUCKET=]

      --s3-region <S3_REGION>
          S3/RustFS region
          
          [env: S3_REGION=]

      --s3-prefix <S3_PREFIX>
          S3/RustFS bucket prefix/folder
          
          [env: S3_PREFIX=]

  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times

      --use-json-logging
          Show logging information as json
          
          [env: USE_JSON_LOGGING=]

      --log-file <LOG_FILE>
          Log file location
          
          [env: LOG_FILE=]

  -h, --help
          Print help (see a summary with '-h')
```
