#!/bin/bash
# Main script that will be run on docker container

# shut the fuck up
# shellcheck disable=SC2164
# shellcheck disable=SC1091

source /root/.bashrc

cd /app/src

cargo build --features unittest
cp /app/src/target/debug/hyraid /app/hyraid-unittest
if ls /app/src/target/debug/hyraid; then
    if /app/scripts/test-all.sh; then
        echo "Unit tests for $1 passed. Compiling regular binary."
        cargo build --release
        cp /app/src/target/release/hyraid /app/bin/"hyraid-$1"
    fi
fi