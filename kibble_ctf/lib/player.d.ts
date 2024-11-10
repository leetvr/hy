/* tslint:disable */
/* eslint-disable */

export type Vec2 = [number, number];
export type Vec3 = [number, number, number];

export interface PlayerState {
  position: Vec3;
  velocity: Vec3;
}

export interface PlayerControls {
  move_direction: Vec2;
  jump: boolean;
  camera_yaw: number; // radians
}

type PlayerUpdate = (current_state: PlayerState, controls: PlayerControls) => PlayerState;
