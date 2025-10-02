# hy - a prototype UGC platform
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://www.mozilla.org/MPL/2.0/)

## Background
This was a prototype game engine that experimented with some novel ideas. It resulted in server side scripting in TypeScript through deno.js, and a functional, performant client written in Rust and compiled to wasm.

It was assmebled in two weeks by:

- [@HexyWitch]
- [@cwfitzgerald]
- [@forginater]
- [@pwc]
- [@kanerogers]

and was funded by the team at [Hytopia](https://hytopia.com).

## Goals
- [ ] Allow web developers to make games with familiar tools

## Preflight checklist
- [ ] npm installed
- [ ] rustup installed
- [ ] rustup target add wasm32-unknown-unknown
- [ ] cargo install wasm-pack

## Getting started
1. Check out the repo
2. Run `./go.sh`
3. Run `curl -vvv -X POST localhost:8888/test_script.js` (currently hangs the event loop)


# License
This project is licensed under the Mozilla Public License 2.0 (MPL-2.0).
See [LICENSE](./LICENSE) for details.
