#!/bin/bash
# Clear out old assets

set -xe

rm -f assets/client_bg*.wasm
rm -f assets/index-*.css
rm -f assets/index-*.js
rm -f assets/index.html

# We want to know if any of these steps fail

# Build the client
(cd client && wasm-pack build --release --target web)
(cd client/ui && npx vite build --minify false)

# Build the scripts
(cd kibble_ctf && npx tsc)

# Copy the resulting files to the assets directory
cp client/ui/dist/assets/* assets/
cp client/ui/dist/index.html assets/

# Start the server
cargo run --release --bin server kibble_ctf
