/* tslint:disable */
/* eslint-disable */
export interface PlayerState {
    x: number;
    y: number;
    z: number;
}

export interface PlayerControls {
    move_x: number;
    move_y: number;
    jump: boolean;
}

export class PlayerControls {
  free(): void;
  jump: boolean;
  move_x: number;
  move_y: number;
}
export class PlayerState {
  free(): void;
}
