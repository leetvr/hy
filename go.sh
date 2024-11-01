#!/bin/bash
# Build the client
(cd client && wasm-pack build --target web)

# Start the server
cargo run --bin server
