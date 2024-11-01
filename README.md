# hy - a prototype UGC platform
## Goals
- [ ] Allow web developers to make games with familiar tools

## Preflight checklist
- [ ] npm installed
- [ ] rustup installed
- [ ] rustup target add wasm32-unknown-unknown
- [ ] cargo binstall wasmpack

## Getting started
1. Check out the repo
2. Run `./go.sh`
3. Run `curl -vvv -X POST localhost:8888/test_script.js` (currently hangs the event loop)
