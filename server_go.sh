#!/bin/bash
set -xe

# Build the scripts
(cd kibble_ctf && npm install && npx tsc)

# Start the server
cargo run --bin server kibble_ctf
