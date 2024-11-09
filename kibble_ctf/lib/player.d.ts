/* tslint:disable */
/* eslint-disable */

export type Vec2 = [number, number];
export type Vec3 = [number, number, number];

export interface PlayerControls {
  move_direction: Vec2;
  jump: boolean;
}

type PlayerUpdate = (position: Vec3, controls: PlayerControls) => Vec3;
