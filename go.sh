#!/bin/bash
# Start the server
cargo build
cargo run --bin server &

# Connect to the server
open https://localhost:8888
