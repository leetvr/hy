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
  animationState: string;
  isOnGround: boolean;
}

export interface PlayerControls {
  readonly move_direction: Vec2;
  readonly jump: boolean;
  readonly fire: boolean;
  readonly camera_yaw: number; // radians
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

export interface Interaction {
  player: number;
  position: Vec3;
  facingAngle: number;
}

type PlayerUpdate = (
  playerID: number,
  currentState: PlayerState,
  controls: PlayerControls,
  collisions: PlayerCollision[],
) => PlayerState;

/**
 * Callback function invoked when an entity is spawned. Useful for changing the model of an entity.
 *
 * @param entityData - The initial data for this entity.
 * @returns The state of this entity when it's first spawned.
 * @remarks
 * Changes to the `entity_type` field will be ignored.
 */
type OnEntitySpawn = (entityData: EntityData) => EntityData;

type EntityUpdate = (
  id: string,
  currentState: EntityState,
  interactions: Interaction[],
) => EntityState;

export const DT = 0.01666667; // 60HZ

interface GlobalHy {
  getEntities: () => Map<String, EntityData[]>;
  spawnEntity: (entity: number, position: Vec3, rotation: Vec3, velocity: Vec3) => String;
  despawnEntity: (entityId: String) => void;
  checkMovementForCollisions: (
    playerID: number,
    currentPosition: Vec3,
    movement: Vec3,
  ) => CollisionResult;
  anchorEntity: (entityId: String, anchorId: number, anchorName: String) => void;
  detachEntity: (entityId: String, position: Vec3) => void;
  interactEntity: (entityId: String, playerId: number, position: Vec3, facingAngle: number) => void;
  getCollisionsForEntity: (entityId: String) => Collision[];
  getCollisionsForPlayer: (playerID: number) => Collision[];
}

interface CollisionResult {
  readonly correctedMovement: Vec3;
  readonly wouldHaveCollided: boolean;
  readonly isOnGround: boolean;
}

interface Collision {
  readonly collisionKind: "Contact" | "Intersection";
  readonly collisionTarget: "Block" | "Entity" | "Player";
  /**
  The ID of the thing this entity collided with  */
  readonly targetId: string;
}

declare global {
  const hy: GlobalHy;
}
