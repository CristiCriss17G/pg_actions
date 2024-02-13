# Postgres database clone script

## Description

This script is used to clone a postgres database . It will create the database if it does not exist.

## Usage

### Docker (recommended)

```bash
docker run -it -v $(pwd)/db_backup:/tmp/psql_backup --rm regcr.ivfuture.uk/devops-services/tools/pg_clone {host} {user} {source_db} {target_db} {user_owner}
```

if you do not wish to save the backup, you can omit the volume mount.

```bash
docker run -it --rm regcr.ivfuture.uk/devops-services/tools/pg_clone {host} {user} {source_db} {target_db} {user_owner}
```

### Local

```bash
./clone_db_postgresql.sh {host} {user} {source_db} {target_db} {user_owner}
```

## Parameters

| Parameter  | Description                                  | ENV variable    |
| ---------- | -------------------------------------------- | --------------- |
| host       | Hostname of the postgres database            | PGHOST          |
| user       | Username of the postgres database            | PGUSER          |
| source_db  | Name of the source database                  | DATABASE_SOURCE |
| target_db  | Name of the target database                  | DATABASE_TARGET |
| user_owner | Username of the owner of the target database | NEW_OWNER       |
| -          | Password of the postgres database            | PASSWORD        |

If the password is not provided, the script will prompt for it, if the session is interactive.
