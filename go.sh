#!/bin/bash
set -xe
# Build the client
(cd client && wasm-pack build --target web)
(cd client/ui && npm install && npx vite build --minify false && cp dist/assets/index*js main.js && cp dist/assets/*css main.css)

# Start the server
cargo run --bin server
