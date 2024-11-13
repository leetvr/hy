#!/bin/bash
set -xe

# Build the scripts
(cd kibble_ctf && npm install && npx tsc)

# Start the server
RUST_LOG=debug RUST_BACKTRACE=1 cargo run --bin server kibble_ctf
