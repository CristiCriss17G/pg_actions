#!/bin/bash

helpFunction() {
    echo ""
    echo "Usage: $0 host user database_source database_target new_owner"
    echo "or set the environment variables PGHOST, PGUSER, DATABASE_SOURCE, DATABASE_TARGET and NEW_OWNER"
    echo "you will be prompted for the password or set the environment variable PASSWORD(not recommended)"
    exit 1 # Exit script after printing help
}

printSettings() {
    echo "PGHOST: $PGHOST"
    echo "PGUSER: $PGUSER"
    echo "DATABASE_SOURCE: $DATABASE_SOURCE"
    echo "DATABASE_TARGET: $DATABASE_TARGET"
    echo "NEW_OWNER: $NEW_OWNER"
    # echo "PASSWORD: $PASSWORD"
    echo "DATABASE_BACKUP_FILE: $DATABASE_BACKUP_FILE"
    echo
}

# Print helpFunction in case first parameter is -h
if [ "$1" == "-h" ]; then
    helpFunction
fi

# check if the session is interactive
if [ -t 0 ]; then
    INTERACTIVE=1
else
    INTERACTIVE=0
fi

# Database connection parameters
# check if HOST is set as argument
if [ -n "$1" ]; then
    PGHOST=$1
elif [ -z "$PGHOST" ]; then
    echo "PGHOST is unset"
    helpFunction
fi

# check if USER is set as argument
if [ -n "$2" ]; then
    PGUSER=$2
elif [ -z "$PGUSER" ]; then
    echo "PGUSER is unset"
    helpFunction
fi

# check if DATABASE_SOURCE is set as argument
if [ -n "$3" ]; then
    DATABASE_SOURCE=$3
elif [ -z "$DATABASE_SOURCE" ]; then
    echo "DATABASE_SOURCE is unset"
    helpFunction
fi

# check if DATABASE_TARGET is set as argument
if [ -n "$4" ]; then
    DATABASE_TARGET=$4
elif [ -z "$DATABASE_TARGET" ]; then
    echo "DATABASE_TARGET is unset"
    helpFunction
fi

# check if NEW_OWNER already is set
if [ -n "$5" ]; then
    NEW_OWNER=$5
elif [ -z "$NEW_OWNER" ]; then
    echo "NEW_OWNER is unset"
    helpFunction
fi

# check if PASSWORD already is set
if [ -z "$PASSWORD" ]; then
    echo "PASSWORD is unset as environment variable"
    # if the session is not interactive exit
    if [ $INTERACTIVE -eq 0 ]; then
        echo "PASSWORD is unset and the session is not interactive"
        exit 1
    fi
    # It's safer to prompt for the password than to hardcode it
    echo "Enter password:"
    read -s PASSWORD
fi

# create a .pgpass file
echo "$PGHOST:5432:*:$PGUSER:$PASSWORD" >.pgpass
chmod 0600 .pgpass
export PGPASSFILE=$(pwd)/.pgpass

# chack if variable RUN_IN_DOCKER is set and set the path for the backupfile in /tmp/psql_backup/
if [ -n "$RUN_IN_DOCKER" ]; then
    mkdir -p /tmp/psql_backup
    DATABASE_BACKUP_FILE=/tmp/psql_backup/$DATABASE_SOURCE.bsql
else
    DATABASE_BACKUP_FILE=$DATABASE_SOURCE.bsql
fi

printSettings

# backup database
echo "Backing up database $DATABASE_SOURCE"
pg_dump -v -h $PGHOST -U $PGUSER -d $DATABASE_SOURCE -f $DATABASE_BACKUP_FILE --format=c --compress=9 --no-privileges --no-owner
# check return code
if [ $? -ne 0 ]; then
    echo "Backup failed"
    exit 1
fi

# check if the target database exists
if psql -h $PGHOST -U $PGUSER -lqt | cut -d \| -f 1 | grep -qw $DATABASE_TARGET; then
    echo "Database $DATABASE_TARGET already exists"
    exit 1
fi

# check if the new owner exists
if ! psql -h $PGHOST -U $PGUSER -tAc "SELECT 1 FROM pg_roles WHERE rolname='$NEW_OWNER'" | grep -q 1; then
    echo "User $NEW_OWNER does not exist"
    exit 1
fi

# create target database
echo "Creating database $DATABASE_TARGET"
createdb -e -h $PGHOST -U $PGUSER -O "$NEW_OWNER" $DATABASE_TARGET
# check return code
if [ $? -ne 0 ]; then
    echo "Creating database failed"
    exit 1
fi

# restore database
echo "Restoring database $DATABASE_TARGET"
pg_restore -v -h $PGHOST -U $PGUSER -d $DATABASE_TARGET --no-owner --format=c $DATABASE_BACKUP_FILE
# check return code
if [ $? -ne 0 ]; then
    echo "Restoring database failed"
    exit 1
fi

echo "Changing owner of database $DATABASE_TARGET to $NEW_OWNER"
# Connect to the database and generate the ALTER TABLE commands
COMMANDS=$(psql -h $PGHOST -U $PGUSER -d $DATABASE_TARGET -t -c "SELECT 'ALTER TABLE \"' || schemaname || '\".\"' || tablename || '\" OWNER TO $NEW_OWNER;' FROM pg_tables WHERE schemaname = 'public';")

# Execute the generated commands
echo "$COMMANDS" | psql -h $PGHOST -U $PGUSER -d $DATABASE_TARGET
# check return code
if [ $? -ne 0 ]; then
    echo "Changing owner of database failed"
    exit 1
fi

# if the session is not interactive check if KEEP_BACKUP is set and 1 to keep the backup file
if [ $INTERACTIVE -eq 0 ]; then
    if [ -z "$KEEP_BACKUP" ]; then
        KEEP_BACKUP=0
    fi
else
    # ask the user if the backup file should be kept
    read -p "Do you want to keep the database backup file? [y/n] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        KEEP_BACKUP=1
    else
        KEEP_BACKUP=0
    fi
fi
# ask the user if the source database file should be deleted
if [ $KEEP_BACKUP -eq 0 ]; then
    rm $DATABASE_BACKUP_FILE
fi

# remove the .pgpass file
rm .pgpass
