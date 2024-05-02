# Postgres database actions

[![pipeline status](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/badges/main/pipeline.svg)](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/commits/main)

[![Latest Release](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/badges/release.svg)](https://gitlab.com/iv_future/infrastructure/docker-tools/postgres-db-actions/-/releases)

## Description

This script is used to perform various actions on a postgres server.

## Usage

### Docker (recommended)

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help


A simple CLI tool for managing Postgres databases

Usage: postgres-db-actions [OPTIONS] [COMMAND]

Commands:
  clone     Clone a database
  backup    Backup operations
  user      User operations
  database  Database operations
  help      Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
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

#### Mount .pgpass file

```bash
docker run -v $HOME/.pgpass:/pghome/.pgpass -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help
```

### Main variables

They can be either set as environment variables or passed as arguments.

- `PG_HOSTNAME` - `-H, --pg-hostname` - Postgres connection URL
- `PG_SUPERUSER` - `-U, --pg-superuser` - Postgres username
- `PG_PASS` - `-P, --pg-password` - Postgres password
- `PG_PORT` - `-p, --pg-port` - Postgres port
- `S3_ENDPOINT` - `--s3-endpoint` - S3/Minio endpoint
- `S3_ACCESS_KEY` - `--s3-access-key` - S3/Minio access key
- `S3_SECRET_KEY` - `--s3-secret-key` - S3/Minio secret key
- `S3_BUCKET` - `--s3-bucket` - S3/Minio bucket
- `S3_REGION` - `--s3-region` - S3/Minio region
- `S3_PREFIX` - `--s3-prefix` - S3/Minio bucket prefix/folder

Also it is recommended to use a `.pgpass` file to store the password; you can store multiple passwords for different hosts and users. It can be mounted as a volume in the container: `-v $HOME/.pgpass:/pghome/.pgpass`.

### Examples

#### Clone a database

##### Help

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help clone

Clone a database

Usage: postgres-db-actions clone [OPTIONS] --database <DATABASE> --new-database <NEW_DATABASE> --new-owner <NEW_OWNER>

Options:
  -d, --database <DATABASE>            database to clone
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -n, --new-database <NEW_DATABASE>    new destination database
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
      --overwrite                      overwrite new database if it exists
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -o, --new-owner <NEW_OWNER>          owner user for db
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
  -c, --create-owner                   create the new owner user
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
  -s, --new-password <NEW_PASSWORD>    new password for db provide if the user does not exist
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
  -k, --keep-dump                      keep the dump file defaults to false [env: KEEP_DUMP=]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --pg-hostname2 <PG_HOSTNAME2>    Optional new host [env: PG_HOSTNAME2=]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --pg-port2 <PG_PORT2>            Optional new port [env: PG_PORT2=]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --pg-superuser2 <PG_SUPERUSER2>  Optional user for new host [env: PG_SUPERUSER2=]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
      --pg-password2 <PG_PASSWORD2>    Optional password for new host [env: PG_PASSWORD2]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

###### Additional Variables

- `PG_HOSTNAME2` - `--pg-hostname2` - Optional new host
- `PG_PORT2` - `--pg-port2` - Optional new port
- `PG_SUPERUSER2` - `--pg-superuser2` - Optional user for new host
- `PG_PASSWORD2` - `--pg-password2` - Optional password for new host

##### Example

- In th same server

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest clone -d old_db -n new_db -o new_owner
```

- Between servers

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -e PG_HOSTNAME2 -e PG_PORT2 -e PG_SUPERUSER2 -e PG_PASSWORD2 -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest clone -d old_db -n new_db -o new_owner
```

#### Backup a database

##### Help

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help backup

Backup operations

Usage: postgres-db-actions backup [OPTIONS] <DATABASE>

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

##### Example

- Backup all databases to S3

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest backup all
```

- Backup a single database to a file, remember to mount the volume

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm -v /path/to/backup:/backup regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest backup my_db -o /backup
```

#### User operations

##### Help

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help user

User operations

Usage: postgres-db-actions user [OPTIONS] [COMMAND]

Commands:
  list    List users
  create  Create a user
  delete  Delete a user
  update  Update a user
  help    Print this message or the help of the given subcommand(s)

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### List users

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest user list
```

Options:

- `--sort <SORT>` - Sort ascending or descending by username
- `-q, --quiet` - Do not print as a table
- `-e, --extra` - Print extra information

##### Create a user

Help:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help user create

Create a user

Usage: postgres-db-actions user create [OPTIONS] --password <PASSWORD> <USERNAME>

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
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

Example:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest user create -s my_password my_user
```

##### Delete a user

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest user delete my_user
```

##### Update a user

Help:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help user update

Update a user

Usage: postgres-db-actions user update [OPTIONS] <USERNAME>

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
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

Example:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest user update -s my_password my_user
```

#### Database operations

##### Help

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help database

Database operations

Usage: postgres-db-actions database [OPTIONS] [COMMAND]

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
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

##### List databases

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest database list
```

Options:

- `--sort <SORT>` - Sort ascending or descending by database name
- `-q, --quiet` - Do not print as a table
- `-e, --extra` - Print extra information

##### Create a database

Help:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help database create

Create a database

Usage: postgres-db-actions database create [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --owner <OWNER>                  owner user for db
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

Example:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest database create -o my_owner my_db
```

##### Delete a database

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest database delete my_db
```

##### Update a database

Help:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest help database update

Update a database

Usage: postgres-db-actions database update [OPTIONS] <DATABASE>

Arguments:
  <DATABASE>  database name

Options:
  -H, --pg-hostname <PG_HOSTNAME>      Sets Postgres connection URL [env: PG_HOSTNAME=db] [default: localhost]
  -o, --new-owner <NEW_OWNER>          new owner user for db
  -U, --pg-superuser <PG_SUPERUSER>    Sets Postgres username [env: PG_SUPERUSER=postgres] [default: postgres]
  -P, --pg-password <PG_PASSWORD>      Sets Postgres password [env: PG_PASS] [default: postgres]
  -p, --pg-port <PG_PORT>              Sets Postgres port [env: PG_PORT=5432] [default: 5432]
      --s3-endpoint <S3_ENDPOINT>      S3/Minio endpoint [env: S3_ENDPOINT=https://miniolocal:9000]
      --s3-access-key <S3_ACCESS_KEY>  S3/Minio access key [env: S3_ACCESS_KEY=minio]
      --s3-secret-key <S3_SECRET_KEY>  S3/Minio secret key [env: S3_SECRET_KEY]
      --s3-bucket <S3_BUCKET>          S3/Minio bucket [env: S3_BUCKET=pgbackups]
      --s3-region <S3_REGION>          S3/Minio region [env: S3_REGION=local]
      --s3-prefix <S3_PREFIX>          S3/Minio bucket prefix/folder [env: S3_PREFIX=pgbackups]
  -v, --verbose...                     Turn debugging information on repetitive use increases verbosity, at most 2 times
  -h, --help                           Print help
```

Example:

```bash
docker run -e PG_HOSTNAME -e PG_SUPERUSER -e PG_PASS -e PG_PORT -e S3_ENDPOINT -e S3_ACCESS_KEY -e S3_SECRET_KEY -e S3_BUCKET -e S3_PREFIX -e S3_REGION -it --rm regcr.ivfuture.uk/devops-services/tools/postgres-db-actions:latest database update -o my_new_owner my_db
```
