/* tslint:disable */
/* eslint-disable */

export type Vec2 = [number, number];
export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];

type CustomState = {
  [key: string]: any;
};

type AnchorName = string;
type EntityId = string;

type AttachedEntities = {
  [key: AnchorName]: EntityId[];
};

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
  customState: CustomState;
  attachedEntities: AttachedEntities;
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

export interface Anchor {
  playerId: number;
  parentAnchor: AnchorName;
}

export interface EntityState {
  position: Vec3;
  velocity: Vec3;
  rotation: Quat;
  anchor: Anchor | null;
  interactions: Interaction[],
  customState: CustomState;
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
) => PlayerState;

type OnPlayerSpawn = (
  playerID: number,
  currentState: PlayerState,
) => PlayerState;

type OnAddPlayer = (
  worldState: CustomState,
  playerID: number,
  currentState: PlayerState,
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
) => EntityState;

export const DT = 0.01666667; // 60HZ

interface GlobalHy {
  getEntities: () => { [key: EntityId]: EntityState };
  getEntityData: (entityId: EntityId) => EntityData;
  spawnEntity: (entity: number, position: Vec3, rotation: Vec3, velocity: Vec3) => EntityId;
  despawnEntity: (entityId: EntityId) => void;
  checkMovementForCollisions: (
    playerID: number,
    currentPosition: Vec3,
    movement: Vec3,
  ) => CollisionResult;
  anchorEntity: (entityId: EntityId, anchorId: number, anchorName: AnchorName) => void;
  detachEntity: (entityId: EntityId, position: Vec3) => void;
  interactEntity: (entityId: EntityId, playerId: number, position: Vec3, facingAngle: number) => void;
  getCollisionsForEntity: (entityId: EntityId) => Collision[];
  getCollisionsForPlayer: (playerID: number) => Collision[];
}

interface CollisionResult {
  readonly correctedMovement: Vec3;
  readonly wouldHaveCollided: boolean;
  readonly isOnGround: boolean;
}

interface Collision {
  readonly collisionKind: "contact" | "intersection";
  readonly collisionTarget: "block" | "entity" | "player";
  /**
  The ID of the thing this entity collided with  */
  readonly targetId: string;
}

declare global {
  const hy: GlobalHy;
}
