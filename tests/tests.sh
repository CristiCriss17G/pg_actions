#!/bin/bash

cargo run -- clone -d dvdrentals -n dvd2 -o abcd -c -s abcdefg

cargo run -- clone -d dvdrentals -n dvd2 -o abcd --overwrite

cargo run backup all -j 2 -vv 2> log.log

cargo run backup all -vv 2> log.log

cargo run backup all