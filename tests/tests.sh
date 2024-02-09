#!/bin/bash

cargo run -- clone -d dvdrentals -n dvd2 -o abcd -c -s abcdefg

cargo run -- clone -d dvdrentals -n dvd2 -o abcd --overwrite