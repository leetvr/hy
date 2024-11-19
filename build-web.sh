#!/bin/bash
set -xe

rm -f assets/client_bg*.wasm
rm -f assets/index-*.css
rm -f assets/index-*.js
rm -f assets/index.html

# Build the client
(cd client && wasm-pack build --release --target web)
(cd client/ui && npm install && npx vite build --minify false)

# Copy the resulting files to the assets directory
cp client/ui/dist/assets/* assets/
cp client/ui/dist/index.html assets/
