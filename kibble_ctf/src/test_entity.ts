import type {
  EntityState,
  EntityUpdate,
  Vec3,
  Interaction,
  OnEntitySpawn,
  EntityData,
} from "../lib/hy";

const ENTITY_SPEED = 15;
const DT = 0.01666667;

export const onSpawn: OnEntitySpawn = (entityData: EntityData): EntityData => {
  if (Math.random() > 0.5) {
    return {
      ...entityData,
      model_path: "kibble_ctf/test_entity_alt.gltf",
    };
  }

  return entityData;
};

export const update: EntityUpdate = (
  currentState: EntityState,
  interactions: Interaction[],
): EntityState => {
  const [lastX, lastY, lastZ] = currentState.position;
  const [velX, velY, velZ] = currentState.velocity;
  const nextPosition: Vec3 = [lastX + velX * DT, lastY + velY * DT, lastZ + velZ * DT];

  if (nextPosition[0] > 32) {
    nextPosition[0] = 0;
  }
  if (nextPosition[0] < 0) {
    nextPosition[0] = 32;
  }
  if (nextPosition[2] > 32) {
    nextPosition[2] = 0;
  }
  if (nextPosition[2] < 0) {
    nextPosition[2] = 32;
  }

  return {
    ...currentState,
    position: nextPosition,
  };
};
