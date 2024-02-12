#!/bin/bash

cargo run -- clone -d dvdrentals -n dvd2 -o abcd -c -s abcdefg

cargo run -- clone -d dvdrentals -n dvd2 -o abcd --overwrite

cargo run backup all -j 2 -vv 2> log.log

cargo run backup all -vv 2> log.log

cargo run backup all

cargo run backup all -vv 2>&1 | tee log.log

cargo run backup all -vv -o ./pgbk.tar.xz 

cargo run backup all -j 10 -vv --retry /tmp/psql_backup/postgresql-backup-2024-02-12-21-24-55.tar.xz  2>&1 | tee log.log