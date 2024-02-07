#!/bin/bash

# This script will seed the dvdrentals database with data from the from the dump file
# located in the same directory as this script.
DATABASE_TARGET=dvdrentals
DATABASE_BACKUP_FILE=dvdrentals.bsql
# if the curent directory is not the same as the script directory add the script directory to the path of the DATABASE_BACKUP_FILE
if [ "$(pwd)" != "$(dirname "$0")" ]; then
    DATABASE_BACKUP_FILE="$(dirname "$0")/$DATABASE_BACKUP_FILE"
fi

# The script will create a new database called dvdrentals and then restore the dump file
# into that database.
createdb -e -h $PG_HOSTNAME -U $POSTGRES_USER $DATABASE_TARGET

# Restore the dump file into the dvdrentals database
pg_restore -v -h $PG_HOSTNAME -U $POSTGRES_USER -d $DATABASE_TARGET --no-owner --format=c $DATABASE_BACKUP_FILE