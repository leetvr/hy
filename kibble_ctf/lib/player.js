import * as wasm from "./player_bg.wasm";
export * from "./player_bg.js";
import { __wbg_set_wasm } from "./player_bg.js";
__wbg_set_wasm(wasm);