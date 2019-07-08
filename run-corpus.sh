#!/bin/sh

set -eux

export RUST_BACKTRACE=1

cd corpus

for corpus in $(find . -mindepth 1 -maxdepth 1 -type d); do
    pushd "$corpus"

    cargo run -- ./*.xml ./ ./out.json

    popd
done
