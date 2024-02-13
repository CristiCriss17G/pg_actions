#!/bin/bash

cargo run -- clone -d dvdrentals -n dvd2 -o abcd -c -s abcdefg

cargo run -- clone -d dvdrentals -n dvd2 -o abcd --overwrite

cargo run backup all -j 2 -vv 2> log.log

cargo run backup all -vv 2> log.log

cargo run backup all

cargo run backup all -vv 2>&1 | tee log.log

cargo run backup all -vv -o ./pgbk.tar.xz 

cargo run backup all -j 10 -vv --retry /tmp/psql_backup/postgresql-backup-2024-02-12-21-24-55.tar.xz  2>&1 | tee log.log

cargo run database -vv delete abcdd6

cargo run database -v list -e --sort asc

cargo run database -vv update dvd3 -o abcdd6

cargo run database -vv list -q

cargo run user create -vv -u abcdd6 -s lmk --superuser

cargo run user update abcdd6 -vv -n true

cargo run user update abcdd6 -vv -n false --superuser=false

cargo run user update abcdd6 -vv -s "newlmk"

cargo run user list -v -e

cargo run user delete abcdd5

cargo run user create -vv -u abcdd7 -s lmk --createdb

cargo run user -vv list --sort asc -q

cargo run user -vv list