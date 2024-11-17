#!/bin/bash
set -xe

# Build the scripts
(cd kibble_ctf && npx tsc)

# Start the server
RUST_BACKTRACE=1 cargo run --release --bin server kibble_ctf
