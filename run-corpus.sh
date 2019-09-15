#!/bin/sh

set -eux

export RUST_BACKTRACE=1

cd corpus

for corpus in $(find . -mindepth 1 -maxdepth 1 -type d); do
    (
        cd "$corpus"

        cargo run -- --json ./*.xml ./ ./out.json.ugly
        jq . < out.json.ugly > out.json
        rm out.json.ugly
    )
done
