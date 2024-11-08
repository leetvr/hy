import { PlayerState, PlayerControls } from "./lib/player";

export function update(controls: PlayerControls, state: PlayerState): PlayerState {
    state.x += controls.move_x * 10.;
    state.y += controls.move_y * 10.;
    return state;
}