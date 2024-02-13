# Postgres database actions

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
