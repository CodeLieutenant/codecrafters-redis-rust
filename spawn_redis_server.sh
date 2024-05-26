#!/bin/sh

exec cargo run \
    --release \
    --target-dir=/tmp/codecrafters-redis-target \
    --manifest-path "$(dirname $0)/Cargo.toml" \
    -- "$@"
