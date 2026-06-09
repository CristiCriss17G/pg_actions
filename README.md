# Postgres database actions

[![CI](https://github.com/CristiCriss17G/pg_actions/actions/workflows/ci.yml/badge.svg)](https://github.com/CristiCriss17G/pg_actions/actions/workflows/ci.yml)

[![Latest Release](https://img.shields.io/github/v/release/CristiCriss17G/pg_actions?label=latest%20release)](https://github.com/CristiCriss17G/pg_actions/releases/latest)

## Contents

- [Description](#description)
- [Installation](#installation)
- [Usage](#usage)
- [Command reference](#command-reference)
- [CI/CD usage](#cicd-usage)
- [License](#license)

## Description

`pg_actions` is a small executable for administrative PostgreSQL database operations. It can clone a database, back up a database, restore a database, create, delete, and update users, create, delete, and update databases, and list users and databases.

## Installation

### Requirements

* `openssl (>= 3.0.0)` - Required for the `pg_actions` executable, it is used for the encryption and decryption of the database connection and S3/RustFS authentication.
* `xz-utils` - Required for the `pg_actions` executable, it is used for the compression and decompression of the backup files.
* `pg_dump`, `pg_restore`, and `psql` `(>=16.0)` - Optional, but recommended for backup and clone operations. If they are not installed, the executable will fail to perform these operations. For installation, see the [Postgres documentation](https://www.postgresql.org/download/).
  * Ubuntu/Debian:

  ```bash
  sudo apt update && sudo apt upgrade -y && sudo apt install gpg wget lsb-release apt-transport-https -y
  wget -qO - https://www.postgresql.org/media/keys/ACCC4CF8.asc | sudo gpg --dearmor -o /usr/share/keyrings/postgresql-archive-keyring.gpg
  echo "deb [signed-by=/usr/share/keyrings/postgresql-archive-keyring.gpg] https://apt.postgresql.org/pub/repos/apt $(lsb_release -cs)-pgdg main" | sudo tee /etc/apt/sources.list.d/pgdg.list
  sudo apt update && sudo apt install -y postgresql-client-16
  ```

  * Alpine:

  ```bash
  apk add postgresql16-client
  ```

### Distribution

There are 3 main ways to get the executable:

* `Docker` - Recommended for CI/CD pipelines, cronjobs, or any other automated process, or unsupported platforms: `ghcr.io/cristicriss17g/pg_actions:latest`
* `Direct executables` - For local development or testing, or local execution without installation. This is also split into 2 subcategories:
  * `pg_actions` - the main executable, smallest, and most efficient, but it relies on a linux system that has `openssl (>= 3.0.0)` and `xz-utils` installed.
  * `pg_actions-static` - the same as `pg_actions` but statically compiled, so it does not rely on the system libraries.
* `Deb package` - For installation on a Debian-based system, it is the most convenient way to install and use the executable. Also split into 2 subcategories:
  * `pg-actions_${RELEASE_VERSION}_amd64.deb` - the main package, it installs the executable, recommended for Ubuntu 22.04 or newer and Debian 11 or newer.
  * `pg-actions-static_${RELEASE_VERSION}_amd64.deb` - the static package, it installs the statically compiled executable, recommended for older systems or systems that do not have the required libraries.

All downloads can be found in the [releases](https://github.com/CristiCriss17G/pg_actions/releases) section.

### Basic usage

#### Docker

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm ghcr.io/cristicriss17g/pg_actions:latest help
```

#### Executables

```bash
./pg_actions help
```

```bash
./pg_actions-static help
```

#### Deb package

*Note: replace `${RELEASE_VERSION}` with the desired version.*

```bash
sudo apt install ./pg-actions_${RELEASE_VERSION}_amd64.deb

pg_actions help
```

> Note: The `pg_actions` executable requires `openssl (>= 3.0.0)` and `xz-utils` to be installed on the system. If they are not installed, the executable will not work, they are usually already installed on systems newer than Ubuntu 22.04 or Debian 11. The `pg_actions-static` executable does not have this requirement.

```bash
sudo apt install ./pg-actions-static_${RELEASE_VERSION}_amd64.deb

pg_actions help
```

> Note 1: Both installers will create the same `pg_actions` executable command, but the `pg_actions-static` will be statically compiled and will not require the system libraries.

> Note 2: The `apt install` method is preferred compared to `dpkg -i` because it will automatically install the dependencies.

## Usage

### Docker run

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm ghcr.io/cristicriss17g/pg_actions:latest help
```

#### Mount .pgpass file

```bash
docker run -v $HOME/.pgpass:/pghome/.pgpass -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm ghcr.io/cristicriss17g/pg_actions:latest help
```

> The flags with `-e` are optional and can be passed as arguments to the command.

### Executables run

```bash
pg_actions help
```

```bash
./pg_actions-static help
```

> The executable can automatically read the `.pgpass` file if it is in the user's home directory.

> For the rest of the documentation, the `pg_actions` command will be used, but it can be replaced with `./pg_actions-static` if the static executable is used; or the docker command.

### Main variables

They can be either set as environment variables or passed as arguments.

Main variables for database connection:

* `PG_HOSTNAME` - `-H, --pg-hostname` - Postgres connection URL
* `PG_SUPERUSER` - `-U, --pg-superuser` - Postgres username
* `PG_PASS` - `-P, --pg-password` - Postgres password
* `PG_PORT` - `-p, --pg-port` - Postgres port

These variables can be simplified by using a `.pgpass` file in the user's home directory. For more information about the `.pgpass` file, see the [Postgres documentation](https://www.postgresql.org/docs/current/libpq-pgpass.html).

Main variables for S3/RustFS, these are required just for the backup operation, but they can be used for all operations that require S3/RustFS storage:

* `S3_ENDPOINT` - `--s3-endpoint` - S3/RustFS endpoint
* `S3_ACCESS_KEY` - `--s3-access-key` - S3/RustFS access key
* `S3_SECRET_KEY` - `--s3-secret-key` - S3/RustFS secret key
* `S3_BUCKET` - `--s3-bucket` - S3/RustFS bucket
* `S3_REGION` - `--s3-region` - S3/RustFS region
* `S3_PREFIX` - `--s3-prefix` - S3/RustFS bucket prefix/folder

Optional variables:

* `PG_DUMP` - `--pg-dump` - Path to the `pg_dump` executable, defaults to `pg_dump` from the system path
* `PG_RESTORE` - `--pg-restore` - Path to the `pg_restore` executable, defaults to `pg_restore` from the system path
* `USE_JSON_LOGGING` - `--use-json-logging` - Use JSON logging, defaults to false

### Authentication

As mentioned previously, the executable can read the `.pgpass` file, but it can also read the `PG_HOSTNAME`, `PG_SUPERUSER`, `PG_PASS`, and `PG_PORT` environment variables, or they can be passed as arguments.
When a `.pgpass` file is read, the environment variable `PG_HOSTNAME` or the equivalent argument still needs to be passed. The `.pgpass` file can contain multiple hostnames, so the executable needs to know which one to use.

#### Example

```bash
# .pgpass file
my_host:5432:*my_db*:my_user:my_password
my_host2:5432:*my_db*:my_user:my_password
```

```bash
pg_actions -H my_host help
```

### Logging and verbosity

The CLI has a verbose mode. It can be used multiple times to increase the verbosity of the output, at most 2 times, but the default level is `info` so some information will always be printed.
The logging information is always printed to stderr, so it can be redirected to a file or to `/dev/null`, while the output is always printed to stdout.
Docker is the exception, as both the logging and the output are printed to stdout, this is a docker limitation.

## Command reference

> The examples assume you are using a `.pgpass` file in the user's home directory, so the commands do not include details about the password or port, but they can be added as arguments.

### Help command

It prints the help message for the main command or for a specific subcommand. Along with arguments and options, it also prints the environment variables that can be used to set the options.

```bash
pg_actions help
```

```bash
A simple CLI tool for managing Postgres databases

Usage: pg_actions [OPTIONS] <COMMAND>

Commands:
  clone        Clone a database [aliases: cl]
  backup       Backup operations
  restore      Restore operations
  user         User operations
  database     Database operations
  completions  Generate shell completions
  check-tools  Do a basic check of the tools This is useful for CI/CD pipelines [aliases: ct]
  help         Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
  -V, --version                        Print version
```

### Clone command

It clones a database within the same server, or between servers. It can also create the owner user if it does not exist.

#### Available aliases

* `clone` - Clone command
* `cl` - Clone command alias

#### Help section

```bash
pg_actions help clone
```

```bash
Clone a database

Usage: pg_actions clone [OPTIONS] --database <DATABASE> --new-database <NEW_DATABASE>

Options:
  -d, --database <DATABASE>            database to clone
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -n, --new-database <NEW_DATABASE>    new destination database
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
      --overwrite                      overwrite new database if it exists
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -o, --new-owner <NEW_OWNER>          owner user for db
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -c, --create-owner                   create the new owner user
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
  -s, --new-password <NEW_PASSWORD>    new password for db provide if the user does not exist
  -k, --keep-dump                      keep the dump file defaults to false [env: KEEP_DUMP=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --pg-hostname2 <PG_HOSTNAME2>    Optional new host [env: PG_HOSTNAME2=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --pg-port2 <PG_PORT2>            Optional new port [env: PG_PORT2=] [default: 5432]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --pg-superuser2 <PG_SUPERUSER2>  Optional user for new host [env: PG_SUPERUSER2=]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --pg-password2 <PG_PASSWORD2>    Optional password for new host [env: PG_PASSWORD2]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `PG_HOSTNAME2` - `--pg-hostname2` - Optional new host
* `PG_PORT2` - `--pg-port2` - Optional new port
* `PG_SUPERUSER2` - `--pg-superuser2` - Optional user for new host
* `PG_PASSWORD2` - `--pg-password2` - Optional password for new host
* `-c, --create-owner` - Create the new owner user
* `-s, --new-password` - New password for the database user just created
* `-k, --keep-dump` - Keep the dump file, defaults to false
* `--overwrite` - Overwrite new database if it exists

#### Example

* In the same server

```bash
pg_actions -H my_host clone -d old_db -n new_db -o new_owner
```

* Between servers

```bash
pg_actions -H my_host clone -d old_db -n new_db -o new_owner --pg-hostname new_host
```

*Note: The credentials for the second host are presumed to be in the `.pgpass` file, or they can be passed as arguments.*

### Backup command

It backs up a database to a file or to S3/RustFS storage.

#### Help section

```bash
pg_actions help backup
```

```bash
Backup operations

Usage: pg_actions backup [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database to backup defaults to all

Options:
  -H, --pg-hostname <PG_HOSTNAME>          Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --output-location <OUTPUT_LOCATION>  output location chose between s3 or a file path, defaults to s3; when s3 is chosen, the output location is
                                           in the bucket specified by the S3_* environment variables with the name of
                                           postgresql-backup-<timestamp>.tar.xz; when a file path is chosen, the output location is the file path
                                           specified, but the extension is .tar.xz [env: BACKUP_LOCATION=]
  -j, --jobs <JOBS>                        Parallel jobs to use defaults to 5 [env: JOBS=] [default: 5]
  -U, --pg-superuser <PG_SUPERUSER>        Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -f, --format <FORMAT>                    Format of the output file defaults to bsql [env: FORMAT=] [default: bsql] [possible values: sql, bsql]
  -P, --pg-password <PG_PASSWORD>          Sets Postgres password [env: PG_PASS]
      --no-archive                         Create archive or just files, has no effect on `all` backups [env: NO_ARCHIVE=]
  -p, --pg-port <PG_PORT>                  Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>                  Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>            Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                        Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>          S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>      S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>      S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>              S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>              S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>              S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                         Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging                   Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>                Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                               Print help
```

##### Additional Variables

> This commands needs the S3/RustFS variables to be set, but they can be passed as arguments.

* `BACKUP_LOCATION` - `-o, --output-location` - Output location, choose between S3 or a file path, defaults to S3
* `JOBS` - `-j, --jobs` - Parallel jobs to use, defaults to 5
* `FORMAT` - `-f, --format` - Format of the output file, defaults to bsql
* `NO_ARCHIVE` - `--no-archive` - Create archive or just files, has no effect on `all` backups

#### Example

* Backup all databases to S3

```bash
pg_actions -H my_host backup all
```

* Backup a single database to a file, remember to mount the volume if you are using Docker

```bash
pg_actions -H my_host backup my_db -o /backup/my_db.tar.xz
```

#### More backup examples

The following examples demonstrate common uses of the backup command with the pg_actions utility. These cases cover backing up all databases or a specific database (dvdrentals) with different formats (sql and bsql) and output options.

1. Backup All Databases to Default Location with SQL Format

```bash
pg_actions -H my_host backup all --format sql
```

This command backs up all databases in SQL format. The output location defaults to S3, as specified by the BACKUP_LOCATION environment variable or its default setting.

1. Backup All Databases to a File with SQL Format

```bash
pg_actions -H my_host backup all --format sql -o ./pgbk.tar.xz
```

This command backs up all databases in SQL format to a file named pgbk.tar.xz in the current directory. The -o option overrides the default S3 output location.

1. Backup All Databases to Default Location with BSQL Format

```bash
pg_actions -H my_host backup all --format bsql
```

This command backs up all databases in BSQL format. The output location defaults to S3, similar to the first command.

1. Backup Specific Database (dvdrentals) to a File with SQL Format Without Creating an Archive

```bash
pg_actions -H my_host backup dvdrentals --format sql -o ./pgbk.sql --no-archive
```

This command backs up the dvdrentals database in SQL format to a file named pgbk.sql in the current directory. The --no-archive option indicates that the output should not be archived, which is relevant for individual database backups.

1. Backup Specific Database (dvdrentals) to a File with BSQL Format Without Creating an Archive

```bash
pg_actions -H my_host backup dvdrentals --format bsql -o ./pgbk.bsql --no-archive
```

This command backs up the dvdrentals database in BSQL format to a file named pgbk.bsql in the current directory, without creating an archive.

1. Backup Specific Database (dvdrentals) to a File with SQL Format

```bash
pg_actions -H my_host backup dvdrentals --format sql -o ./pgbk.tar.xz
```

This command backs up the dvdrentals database in SQL format to a file named pgbk.tar.xz in the current directory. This case does not specify the --no-archive option, so the output may be archived depending on the utility's behavior.

1. Backup Specific Database (dvdrentals) to a File with BSQL Format

```bash
pg_actions -H my_host backup dvdrentals --format bsql -o ./pgbk.tar.xz
```

This command backs up the dvdrentals database in BSQL format to a file named pgbk.tar.xz in the current directory. Similar to the previous case, the absence of the --no-archive option means the output may be archived.

### Restore command

It restores a database from a file or from S3/RustFS storage.

#### Help section

```bash
pg_actions help restore
```

```bash
Restore operations

Usage: pg_actions restore [OPTIONS] --input-location <INPUT_LOCATION> <DATABASE>

Arguments:
  <DATABASE>  database to restore

Options:
  -H, --pg-hostname <PG_HOSTNAME>        Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -i, --input-location <INPUT_LOCATION>  input location chose between s3 or a file path when s3 is chosen, the input location should be in the format
                                         s3://bucket_name/file_path you can also specify all the S3_* args/environment variables to use S3, the file
                                         url will have priority [env: RESTORE_LOCATION=]
  -j, --jobs <JOBS>                      Parallel jobs to use defaults to 5 [env: JOBS=] [default: 5]
  -U, --pg-superuser <PG_SUPERUSER>      Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -f, --format <FORMAT>                  Format of the input file defaults to bsql [env: FORMAT=] [default: bsql] [possible values: sql, bsql]
  -P, --pg-password <PG_PASSWORD>        Sets Postgres password [env: PG_PASS]
      --file <FILE_NAME>                 File name, without extension, in case of archive extraction and the file name is different from the database
                                         name [env: FILE_NAME=]
  -p, --pg-port <PG_PORT>                Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -o, --owner <OWNER>                    Owner user for db
      --pg-dump <PG_DUMP>                Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --overwrite                        overwrite the database if it exists [env: OVERWRITE=]
      --pg-restore <PG_RESTORE>          Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
  -k, --keep-temp                        keep the temp files defaults to false [env: KEEP_TEMP=]
      --psql <PSQL>                      Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>        S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>    S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>    S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>            S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>            S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>            S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                       Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging                 Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>              Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                             Print help
```

##### Additional Variables

* `RESTORE_LOCATION` - `-i, --input-location` - Input location, choose between S3 or a file path
* `JOBS` - `-j, --jobs` - Parallel jobs to use, defaults to 5
* `FORMAT` - `-f, --format` - Format of the input file, defaults to bsql
* `FILE_NAME` - `--file` - File name, without extension, in case of archive extraction and the file name is different from the database name
* `OVERWRITE` - `--overwrite` - Overwrite the database if it exists
* `KEEP_TEMP` - `--keep-temp` - Keep the temp files, defaults to false
* `OWNER` - `-o, --owner` - Owner user for db

#### Example

1. Restore a database from S3

```bash
pg_actions -H my_host restore my_db -i s3://bucket_name/file_path
```

1. Restore a database from a file

```bash
pg_actions -H my_host restore my_db -i ./my_db.tar.xz
```

1. Restore a database from a file with a different name

```bash
pg_actions -H my_host restore my_db -i ./my_db.tar.xz --file my_db
```

1. Restore a database from a file with a different name and overwrite the existing database

```bash
pg_actions -H my_host restore my_db -i ./my_db.tar.xz --file my_db --overwrite
```

1. Restore a database from a file with a different name, overwrite the existing database, and keep the temp files

```bash
pg_actions -H my_host restore my_db -i ./my_db.tar.xz --file my_db --overwrite --keep-temp
```

1. Restore a database from a file with a different name, overwrite the existing database, keep the temp files, and change the owner

```bash
pg_actions -H my_host restore my_db -i ./my_db.tar.xz --file my_db --overwrite --keep-temp --owner new_owner
```

1. Restore a database from a file with a different name, overwrite the existing database, keep the temp files, and change the owner with a different format

```bash
pg_actions -H my_host restore my_db -i ./my_db.sql --format sql --overwrite
```

1. Restore a database from a file with a different name, overwrite the existing database, keep the temp files, and change the owner with a different format

```bash
pg_actions -H my_host restore my_db -i ./my_db.bsql --format bsql --overwrite
```

1. Restore a database from a file with a different name, overwrite the existing database, keep the temp files, and change the owner with a different format

```bash
pg_actions -H my_host restore my_db -i ./my_db.sql --format sql --overwrite --keep-temp
```

1. Restore a database from a file with a different name, overwrite the existing database, keep the temp files, and change the owner with a different format

```bash
pg_actions -H my_host restore my_db -i ./my_db.bsql --format bsql --overwrite --keep-temp
```

### User operations

This command allows the user to list, create, delete, and update a user, and to grant and revoke privileges from a user.

#### Help section

```bash
pg_actions help user
```

```bash
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
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

#### List users

Prints a list of user names as a table

##### Help

```bash
pg_actions help user list
```

```bash
List users

Usage: pg_actions user list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
      --sort <SORT>                    sort by username [possible values: asc, desc]
  -o, --output <OUTPUT>                quiet mode [default: table] [possible values: json, json-compact, json-lines, csv, table, simple]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -e, --extra                          extra details
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -q, --query <QUERY>                  query string, fuzzy search
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `--sort [asc|desc]` - Sort ascending or descending by username
* `-o, --output [json|json-compact|json-lines|csv|table|simple]` - Output format, defaults to table
* `-e, --extra` - Print extra information, such as right as superuser, createdb, and login, and oid
* `-q, --query` - Query string, fuzzy search

##### Output formats

* `json` - JSON pretty format
* `json-compact` - Compact JSON format
* `json-lines` - JSON lines format
* `csv` - CSV format
* `table` - Table format
* `simple` - Simple/compact format

##### Execution

```bash
pg_actions -H my_host user list
```

```bash
pg_actions -H my_host user list --sort desc -o json
```

```bash
pg_actions -H my_host user list --sort desc -o csv
```

#### Create a user

Create a user with the specified options

##### Help

```bash
pg_actions help user create
```

```bash
Create a user

Usage: pg_actions user create [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -s, --password <PASSWORD>            password
      --superuser                      user as superuser
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
      --createdb                       createdb for user
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -n, --no-login                       role with no login defaults to false
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `-s, --password` - Password for the user, if not set, a random 32 character password will be generated and printed
* `--superuser` - User as superuser
* `--createdb` - Createdb for user
* `-n, --no-login` - Role with no login, defaults to false

##### Execution

```bash
pg_actions -H my_host user create -s my_password my_user
```

```bash
pg_actions -H my_host user create --superuser my_user
```

#### Delete a user

Delete a user with the specified username

##### Help

```bash
pg_actions help user delete
```

```bash
Delete a user

Usage: pg_actions user delete [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions -H my_host user delete my_user
```

#### Update a user

Update a user with the specified options

##### Help

```bash
pg_actions help user update
```

```bash
Update a user

Usage: pg_actions user update [OPTIONS] <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -s, --password <PASSWORD>            password for user
      --superuser <SUPERUSER>          user as superuser [possible values: true, false]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
      --createdb <CREATEDB>            createdb for user [possible values: true, false]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -n, --no-login <NO_LOGIN>            role with no login [possible values: true, false]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `-s, --password` - Password for the user
* `--superuser` - User as superuser
* `--createdb` - Createdb for user
* `-n, --no-login` - Role with no login, defaults to false

##### Execution

```bash
pg_actions -H my_host user update -s my_password my_user
```

#### Grant privileges to a user

Grant privileges to a user on a database, currently supported, read-only or full access.

##### Help

```bash
pg_actions help user grant
```

```bash
Grant privileges to a user

Usage: pg_actions user grant [OPTIONS] --database <DATABASE> --privileges <PRIVILEGES> <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -d, --database <DATABASE>            database
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
      --schema <SCHEMA>                schema [default: public]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -g, --privileges <PRIVILEGES>        privileges full or read-only or read-update-only [possible values: full, read-only, read-update-only]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `--schema` - Schema, defaults to public
* `-g, --privileges` - Privileges, full or read-only, or read-update-only

##### Execution

```bash
pg_actions -H my_host user grant -d my_db -g full my_user
```

```bash
pg_actions -H my_host user grant -d my_db -g read-only my_user
```

#### Revoke privileges from a user

Revoke privileges from a user on a database, currently supported, read-only or full access, the reverse of the grant command.

##### Help

```bash
pg_actions help user revoke
```

```bash
Revoke privileges from a user

Usage: pg_actions user revoke [OPTIONS] --database <DATABASE> --privileges <PRIVILEGES> <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -d, --database <DATABASE>            database
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
      --schema <SCHEMA>                schema [default: public]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -g, --privileges <PRIVILEGES>        privileges [possible values: full, read-only, read-update-only]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `--schema` - Schema, defaults to public
* `-g, --privileges` - Privileges, full or read-only, or read-update-only

##### Execution

```bash
pg_actions -H my_host user revoke -d my_db -g full my_user
```

```bash
pg_actions -H my_host user revoke -d my_db -g read-only my_user
```

### Database operations

This command allows the user to list, create, delete, and update a database.

#### Help section

```bash
pg_actions help database
```

```bash
Database operations

Usage: pg_actions database [OPTIONS] <COMMAND>

Commands:
  list       List databases
  create     Create a database
  delete     Delete a database
  update     Update a database
  extension  Manipulate extensions on a database [aliases: ext]
  help       Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

#### List databases

Prints a list of databases as a table

##### Help

```bash
pg_actions help database list
```

```bash
List databases

Usage: pg_actions database list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
      --sort <SORT>                    sort by database name [possible values: asc, desc]
  -o, --output <OUTPUT>                quiet mode [default: table] [possible values: json, json-compact, json-lines, csv, table, simple]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -e, --extra                          extra details
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -q, --query <QUERY>                  query string, fuzzy search
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `--sort [asc|desc]` - Sort ascending or descending by database name
* `-o, --output [json|json-compact|json-lines|csv|table|simple]` - Output format, defaults to table
* `-e, --extra` - Print extra information, such as owner, size and oid
* `-q, --query` - Query string, fuzzy search

##### Output formats

* `json` - JSON pretty format
* `json-compact` - Compact JSON format
* `json-lines` - JSON lines format
* `csv` - CSV format
* `table` - Table format
* `simple` - Simple/compact format

##### Execution

```bash
pg_actions -H my_host database list
```

```bash
pg_actions -H my_host database list --sort desc -o json
```

```bash
pg_actions -H my_host database list --sort desc -o csv
```

#### Create a database

Create a database with the specified options

##### Help

```bash
pg_actions help database create
```

```bash
Create a database

Usage: pg_actions database create [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --owner <OWNER>                  owner user for db
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `-o, --owner` - Owner user for the database, must exist, default to the superuser

##### Execution

```bash
pg_actions -H my_host database create my_db
```

#### Delete a database

Delete a database with the specified name

##### Help

```bash
pg_actions help database delete
```

```bash
Delete a database

Usage: pg_actions database delete [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions -H my_host database delete my_db
```

#### Update a database

Update a database with the specified options

##### Help

```bash
pg_actions help database update
```

```bash
Update a database

Usage: pg_actions database update [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --new-owner <NEW_OWNER>          new owner user for db
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `-o, --new-owner` - New owner user for the database, must exist

##### Execution

```bash
pg_actions -H my_host database update -o my_new_owner my_db
```

#### Extension operations

This command allows the user to list, create, delete, and update an extension on a database.

##### Available aliases

* `database ext` - Alias for `database extension`
* `database extension` - Alias for `database extension`

##### Help

```bash
pg_actions help database extension
```

```bash
Manipulate extensions on a database

Usage: pg_actions database extension [OPTIONS] <DATABASE> <COMMAND>

Commands:
  list    List extensions
  create  Create an extension
  delete  Delete an extension
  help    Print this message or the help of the given subcommand(s)

Arguments:
  <DATABASE>  Database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

#### List extensions

Prints a list of extensions on a database as a table

##### Help

```bash
pg_actions help database extension list
```

```bash
List extensions

Usage: pg_actions database extension <DATABASE> list [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
      --sort <SORT>                    sort by extension name [possible values: asc, desc]
  -o, --output <OUTPUT>                quiet mode [default: table] [possible values: json, json-compact, json-lines, csv, table, simple]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -e, --extra                          extra details
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Additional Variables

* `--sort [asc|desc]` - Sort ascending or descending by extension name
* `-o, --output [json|json-compact|json-lines|csv|table|simple]` - Output format, defaults to table
* `-e, --extra` - Print extra information, such as owner, size and oid

##### Execution

```bash
pg_actions -H my_host database extension my_db list
```

```bash
pg_actions -H my_host database extension my_db list --sort desc -o json
```

```bash
pg_actions -H my_host database extension my_db list --sort desc -o csv
```

#### Create an extension

Create an extension on a database

##### Help

```bash
pg_actions help database extension create
```

```bash
Create an extension

Usage: pg_actions database extension <DATABASE> create [OPTIONS] <EXTENSION>

Arguments:
  <EXTENSION>  extension name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions -H my_host database extension my_db create my_extension
```

#### Delete an extension

Delete an extension from a database

##### Help

```bash
pg_actions help database extension delete
```

```bash
Delete an extension

Usage: pg_actions database extension <DATABASE> delete [OPTIONS] <EXTENSION>

Arguments:
  <EXTENSION>  extension name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions -H my_host database extension my_db delete my_extension
```

### CLI completion

The CLI completion is available for bash, elvish, fish, powershell and zsh.

#### Help section

```bash
pg_actions help completions
```

```bash
Generate shell completions

Usage: pg_actions completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  The shell to generate the script for [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

* `bash`

  ```bash
  pg_actions completions bash

  # Add this line to your .bashrc or .bash_profile
  source <(pg_actions completions bash)
  ```

* `elvish`

  ```bash
  pg_actions completions elvish

  # Add this line to your .elvish/rc.elv
  eval (pg_actions completions elvish)
  ```

* `fish`

  ```bash
  pg_actions completions fish

  # Add this line to your config.fish, or create a file in the completions directory
  pg_actions completions fish > ~/.config/fish/completions/pg_actions.fish
  ```

* `powershell`

  ```powershell
  pg_actions completions powershell

  # Add this line to your profile.ps1 or $PROFILE
  pg_actions completions powershell | Out-String | Invoke-Expression
  ```

* `zsh`

  ```bash
  pg_actions completions zsh

  # Add this line to your .zshrc
  source <(pg_actions completions zsh)
  ```

### Tools checking

This command checks the tools needed for the CLI to work, such as `pg_dump` and `pg_restore`.

#### Available aliases

* `check-tools` - Alias for `check tools`
* `ct` - Alias for `check tools`

#### Help section

```bash
pg_actions help check-tools
```

```bash
Do a basic check of the tools This is useful for CI/CD pipelines

Usage: pg_actions check-tools [OPTIONS]

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --psql <PSQL>                    Optional custom psql path If not set, the default from PATH will be used [env: PSQL=]
      --s3-endpoint <S3_ENDPOINT>      S3/RustFS endpoint [env: S3_ENDPOINT=https://rustfslocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/RustFS access key [env: S3_ACCESS_KEY=rustfsadmin]
      --s3-secret-key <S3_SECRET_KEY>  S3/RustFS secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/RustFS bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/RustFS region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/RustFS bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
      --use-json-logging               Show logging information as json [env: USE_JSON_LOGGING=]
      --log-file <LOG_FILE>            Log file location [env: LOG_FILE=pg_actions.log]
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions check-tools
```

## CI/CD usage

The CLI can be used in CI/CD pipelines to automate database operations, such as creating databases, cloning databases, managing users, and granting privileges.

### GitHub Actions

```yaml
name: Clone database

on:
  workflow_dispatch:

jobs:
  clone:
    runs-on: ubuntu-latest
    steps:
      - name: Clone database
        env:
          PG_HOSTNAME: ${{ secrets.PG_HOSTNAME }}
          PG_PORT: ${{ secrets.PG_PORT }}
          PG_SUPERUSER: ${{ secrets.PG_SUPERUSER }}
          PG_PASS: ${{ secrets.PG_PASS }}
        run: |
          docker run --rm \
            -e PG_HOSTNAME \
            -e PG_PORT \
            -e PG_SUPERUSER \
            -e PG_PASS \
            ghcr.io/cristicriss17g/pg_actions:latest \
            clone -d my_db -n my_new_db -o my_user
```

This assumes that the variables `PG_HOSTNAME`, `PG_PORT` (optional), `PG_SUPERUSER`, and `PG_PASS` are set as repository or organization secrets.

### GitLab CI/CD

```yaml
stages:
  - clone

clone:
  stage: clone
  image: 
    name: ghcr.io/cristicriss17g/pg_actions:latest
    entrypoint: [""]
  script:
    - pg_actions -H my_host clone -d my_db -n my_new_db -o my_user
```

This assumes that the variables `PG_HOSTNAME`, `PG_PORT` (optional), `PG_SUPERUSER`, and `PG_PASS` are set in the environment.

## License

`pg_actions` is licensed under the GNU Affero General Public License v3.0 only. See [LICENSE](LICENSE) for the full license text.
