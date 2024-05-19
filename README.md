# Postgres database actions

[![pipeline status](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/badges/main/pipeline.svg)](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/commits/main)

[![Latest Release](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/badges/release.svg)](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/releases)

## Description

`pg_actions` is a small executable that allows various operation on a PostgreSQL database, form the administrative side. It can clone a database, backup a database, create, delete, update a user, create, delete, update a database, and list users and databases.

## Installation

### Requirements

- `openssl (>= 3.0.0)` - Required for the `pg_actions` executable, it is used for the encryption and decryption of the database connection and S3/Minio authentication.
- `xz-utils` - Required for the `pg_actions` executable, it is used for the compression and decompression of the backup files.
- `pg_dump` and `pg_restore` `(>=16.0)` - Optional, but recommended, they are used for the backup and clone operations, if they are not installed, the executable will fail to perform these operations. For installation, see the [Postgres documentation](https://www.postgresql.org/download/).

### Distribution

There are 3 main ways to get the executable:

- `Docker` - Recommended for CI/CD pipelines, cronjobs, or any other automated process, or unsupported platforms: `regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest`
- `Direct executables` - For local development or testing, or local execution without installation. This is also split into 2 subcategories:
  - `pg_actions` - the main executable, smallest, and most efficient, but it relies on a linux system that has `openssl (>= 3.0.0)` and `xz-utils` installed.
  - `pg_actions-static` - the same as `pg_actions` but statically compiled, so it does not rely on the system libraries.
- `Deb package` - For installation on a Debian-based system, it is the most convenient way to install and use the executable. Also split into 2 subcategories:
  - `pg-actions_${RELEASE_VERSION}_amd64.deb` - the main package, it installs the executable, recommended for Ubuntu 22.04 or newer and Debian 11 or newer.
  - `pg-actions-static_${RELEASE_VERSION}_amd64.deb` - the static package, it installs the statically compiled executable, recommended for older systems or systems that do not have the required libraries.

All downloads can be found in the [releases](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/releases) section.

### Utilization

#### Docker

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help
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

> Note 2: The `apt install` method is proffered compared to `dpkg -i` because it will automatically install the dependencies.

## Usage

### Docker run

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help
```

#### Mount .pgpass file

```bash
docker run -v $HOME/.pgpass:/pghome/.pgpass -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help
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

- `PG_HOSTNAME` - `-H, --pg-hostname` - Postgres connection URL
- `PG_SUPERUSER` - `-U, --pg-superuser` - Postgres username
- `PG_PASS` - `-P, --pg-password` - Postgres password
- `PG_PORT` - `-p, --pg-port` - Postgres port

These variables can be simplified by using a `.pgpass` file in the user's home directory. For more information about the `.pgpass` file, see the [Postgres documentation](https://www.postgresql.org/docs/current/libpq-pgpass.html).

Main variables for S3/Minio, these are required just for the backup operation, but they can be used for all operations that require S3/Minio storage:

- `S3_ENDPOINT` - `--s3-endpoint` - S3/Minio endpoint
- `S3_ACCESS_KEY` - `--s3-access-key` - S3/Minio access key
- `S3_SECRET_KEY` - `--s3-secret-key` - S3/Minio secret key
- `S3_BUCKET` - `--s3-bucket` - S3/Minio bucket
- `S3_REGION` - `--s3-region` - S3/Minio region
- `S3_PREFIX` - `--s3-prefix` - S3/Minio bucket prefix/folder

Optional variables:

- `PG_DUMP` - `--pg-dump` - Path to the `pg_dump` executable, defaults to `pg_dump` from the system path
- `PG_RESTORE` - `--pg-restore` - Path to the `pg_restore` executable, defaults to `pg_restore` from the system path

### Mention

The cli has a verbose mode, it can be used multiple times to increase the verbosity of the output, at most 2 times, but the default level is `info` so some information will always be printed.
The logging information is always printed to stderr, so it can be redirected to a file or to `/dev/null`, while the output is always printed to stdout.
Docker is the exception, as both the logging and the output are printed to stdout, this is a docker limitation.

## CLI structure and examples

> The examples assume you are using a `.pgpass` file in the user's home directory, so the commands do not include details about the password or port, but they can be added as arguments.

### Help command

It prints the help message for the main command or for a specific subcommand. Along with arguments and options, it also prints the environment variables that can be used to set the options.

```bash
pg_actions help
```

```bash
A simple CLI tool for managing Postgres databases

Usage: pg_actions [OPTIONS] [COMMAND]

Commands:
  clone        Clone a database
  backup       Backup operations
  user         User operations
  database     Database operations
  completions  Generate shell completions
  check-tools  Do a basic check of the tools This is useful for CI/CD pipelines
  help         Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
  -V, --version                        Print version
```

### Clone command

It clones a database within the same server, or between servers. It can also create the owner user if it does not exist.

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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -o, --new-owner <NEW_OWNER>          owner user for db [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -c, --create-owner                   create the new owner user
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
  -s, --new-password <NEW_PASSWORD>    new password for db provide if the user does not exist
  -k, --keep-dump                      keep the dump file defaults to false [env: KEEP_DUMP=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --pg-hostname2 <PG_HOSTNAME2>    Optional new host [env: PG_HOSTNAME2=]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --pg-port2 <PG_PORT2>            Optional new port [env: PG_PORT2=] [default: 5432]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --pg-superuser2 <PG_SUPERUSER2>  Optional user for new host [env: PG_SUPERUSER2=]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --pg-password2 <PG_PASSWORD2>    Optional password for new host [env: PG_PASSWORD2]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `PG_HOSTNAME2` - `--pg-hostname2` - Optional new host
- `PG_PORT2` - `--pg-port2` - Optional new port
- `PG_SUPERUSER2` - `--pg-superuser2` - Optional user for new host
- `PG_PASSWORD2` - `--pg-password2` - Optional password for new host
- `-c, --create-owner` - Create the new owner user
- `-s, --new-password` - New password for the database user just created
- `-k, --keep-dump` - Keep the dump file, defaults to false
- `--overwrite` - Overwrite new database if it exists

#### Example

- In the same server

```bash
pg_actions clone -d old_db -n new_db -o new_owner
```

- Between servers

```bash
pg_actions clone -d old_db -n new_db -o new_owner -H new_host
```

### Backup command

It backs up a database to a file or to S3/Minio storage.

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
  -H, --pg-hostname <PG_HOSTNAME>
          Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --output-location <OUTPUT_LOCATION>
          output location chose between s3 or a file path, defaults to s3; when s3 is chosen, the output location is in the bucket specified by the S3_* environment variables with the name of postgresql-backup-<timestamp>.tar.xz; when a file path is chosen, the output location is the file path specified, but the extension is .tar.xz [env: BACKUP_LOCATION=]
  -j, --jobs <JOBS>
          Parallel jobs to use defaults to 5 [env: JOBS=] [default: 5]
  -U, --pg-superuser <PG_SUPERUSER>
          Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>
          Sets Postgres password [env: PG_PASS] [default: postgres]
      --retry <RETRY>
          retry just archive upload, Path to the archive to retry from
  -p, --pg-port <PG_PORT>
          Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -s, --password <PASSWORD>
          owner password for db
      --pg-dump <PG_DUMP>
          Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>
          Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>
          S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>
          S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>
          S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>
          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>
          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>
          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...
          Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help
          Print help
```

##### Additional Variables

> This commands needs the S3/Minio variables to be set, but they can be passed as arguments.

- `BACKUP_LOCATION` - `-o, --output-location` - Output location, choose between S3 or a file path, defaults to S3
- `JOBS` - `-j, --jobs` - Parallel jobs to use, defaults to 5
- `RETRY` - `--retry` - Retry just archive upload, path to the archive to retry from

#### Example

- Backup all databases to S3

```bash
pg_actions backup all
```

- Backup a single database to a file, remember to mount the volume if you are using Docker

```bash
pg_actions backup my_db -o /backup
```

### User operations

This command allows the user to list, create, delete, and update a user, and to grant and revoke privileges from a user.

#### Help section

```bash
pg_actions help user
```

```bash
User operations

Usage: pg_actions user [OPTIONS] [COMMAND]

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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
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
  -q, --quiet                          quite mode
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -e, --extra                          extra details
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `--sort [asc|desc]` - Sort ascending or descending by username
- `-q, --quiet` - Do not print as a table
- `-e, --extra` - Print extra information, such as right as superuser, createdb, and login, and oid

##### Execution

```bash
pg_actions user list
```

#### Create a user

Create a user with the specified options

##### Help

```bash
pg_actions help user create
```

```bash
Create a user

Usage: pg_actions user create [OPTIONS] --password <PASSWORD> <USERNAME>

Arguments:
  <USERNAME>  username

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -s, --password <PASSWORD>            password
      --superuser                      user as superuser
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
      --createdb                       createdb for user
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -n, --no-login                       role with no login defaults to false
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `-s, --password` - Password for the user
- `--superuser` - User as superuser
- `--createdb` - Createdb for user
- `-n, --no-login` - Role with no login, defaults to false

##### Execution

```bash
pg_actions user create -s my_password my_user
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions user delete my_user
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -n, --no-login <NO_LOGIN>            role with no login [possible values: true, false]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `-s, --password` - Password for the user
- `--superuser` - User as superuser
- `--createdb` - Createdb for user
- `-n, --no-login` - Role with no login, defaults to false

##### Execution

```bash
pg_actions user update -s my_password my_user
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
  -g, --privileges <PRIVILEGES>        privileges full or read-only [possible values: full, read-only]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `--schema` - Schema, defaults to public
- `-g, --privileges` - Privileges, full or read-only

##### Execution

```bash
pg_actions user grant -d my_db -g full my_user
```

```bash
pg_actions user grant -d my_db -g read-only my_user
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
  -g, --privileges <PRIVILEGES>        privileges [possible values: full, read-only]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `--schema` - Schema, defaults to public
- `-g, --privileges` - Privileges, full or read-only

##### Execution

```bash
pg_actions user revoke -d my_db -g full my_user
```

```bash
pg_actions user revoke -d my_db -g read-only my_user
```

### Database operations

This command allows the user to list, create, delete, and update a database.

#### Help section

```bash
pg_actions help database
```

```bash
Database operations

Usage: pg_actions database [OPTIONS] [COMMAND]

Commands:
  list    List databases
  create  Create a database
  delete  Delete a database
  update  Update a database
  help    Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
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
  -q, --quiet                          quite mode
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -e, --extra                          extra details
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `--sort [asc|desc]` - Sort ascending or descending by database name
- `-q, --quiet` - Do not print as a table
- `-e, --extra` - Print extra information, such as owner, size and oid

##### Execution

```bash
pg_actions database list
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `-o, --owner` - Owner user for the database, must exist, default to the superuser

##### Execution

```bash
pg_actions database create my_db
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions database delete my_db
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Additional Variables

- `-o, --new-owner` - New owner user for the database, must exist

##### Execution

```bash
pg_actions database update -o my_new_owner my_db
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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Execution

- `bash`

  ```bash
  pg_actions completions bash

  # Add this line to your .bashrc or .bash_profile
  source <(pg_actions completions bash)
  ```

- `elvish`

  ```bash
  pg_actions completions elvish

  # Add this line to your .elvish/rc.elv
  eval (pg_actions completions elvish)
  ```

- `fish`

  ```bash
  pg_actions completions fish

  # Add this line to your config.fish, or create a file in the completions directory
  pg_actions completions fish > ~/.config/fish/completions/pg_actions.fish
  ```

- `powershell`

  ```powershell
  pg_actions completions powershell

  # Add this line to your profile.ps1 or $PROFILE
  pg_actions completions powershell | Out-String | Invoke-Expression
  ```

- `zsh`

  ```bash
  pg_actions completions zsh

  # Add this line to your .zshrc
  source <(pg_actions completions zsh)
  ```

### Tools checking

This command checks the tools needed for the CLI to work, such as `pg_dump` and `pg_restore`.

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
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --pg-dump <PG_DUMP>              Optional custom pg_dump path If not set, the default from PATH will be used [env: PG_DUMP=]
      --pg-restore <PG_RESTORE>        Optional custom pg_restore path If not set, the default from PATH will be used [env: PG_RESTORE=]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### Execution

```bash
pg_actions check-tools
```