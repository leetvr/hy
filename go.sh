#!/bin/bash
# Clear out old assets
rm assets/client_bg*.wasm
rm assets/index-*.css
rm assets/index-*.js
rm assets/index.html

# We want to know if any of these steps fail
set -xe

# Build the client
(cd client && wasm-pack build --dev --target web)
(cd client/ui && npm install && npx vite build --minify false)

# Build the scripts
(cd kibble_ctf && npx tsc)

# Copy the resulting files to the assets directory
cp client/ui/dist/assets/* assets/
cp client/ui/dist/index.html assets/

# Start the server
cargo run --bin server kibble_ctf
