import { PlayerState } from "./lib/player";

function updatePlayerState(playerState: PlayerState): PlayerState {
    playerState.position += 1;
    return playerState;
}