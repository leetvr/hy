/* tslint:disable */
/* eslint-disable */

export type Vec2 = [number, number];
export type Vec3 = [number, number, number];

export interface BlockPos {
  x: number;
  y: number;
  z: number;
}

export interface PlayerState {
  position: Vec3;
  velocity: Vec3;
}

export interface PlayerControls {
  move_direction: Vec2;
  jump: boolean;
  camera_yaw: number; // radians
}

export interface EntityData {
  name: string;
  entity_type: number;
  model_path: string;
  state: EntityState;
}

export interface EntityState {
  position: Vec3;
  velocity: Vec3;
}

export interface PlayerCollision {
  block: BlockPos;
  normal: Vec3;
  resolution: Vec3;
}

type PlayerUpdate = (
  current_state: PlayerState,
  controls: PlayerControls,
  collisions: PlayerCollision[],
) => PlayerState;
type EntityUpdate = (current_state: EntityState) => EntityState;

export const DT = 0.01666667; // 60HZ

interface GlobalHy {
  getEntities: () => Map<String, EntityData[]>;
}

declare global {
  const hy: GlobalHy;
}
