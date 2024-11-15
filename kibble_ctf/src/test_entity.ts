import type { EntityData, EntityState, EntityUpdate, OnEntitySpawn, Vec3 } from "../lib/hy";

const ENTITY_SPEED = 15;
const DT = 0.01666667;

export const onSpawn: OnEntitySpawn = (entityData: EntityData): EntityData => {
  if (Math.random() > 0.5) {
    console.log("[onspawn] Lucky!");
    return {
      ...entityData,
      model_path: "kibble_ctf/test_entity_alt.gltf",
    };
  }

  console.log("[onspawn] Unlucky!");
  return entityData;
};

export const update: EntityUpdate = (currentState: EntityState): EntityState => {
  const [lastX, lastY, lastZ] = currentState.position;
  const nextPosition: Vec3 = [lastX, lastY, lastZ - ENTITY_SPEED * DT];

  if (nextPosition[2] < 0) {
    // Reset the current entity
    nextPosition[2] = 32;
  }

  return {
    ...currentState,
    position: nextPosition,
  };
};
