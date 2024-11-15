import type { EntityData, EntityState, EntityUpdate, OnEntitySpawn, Vec3 } from "../lib/hy";

const ENTITY_SPEED = 15;
const DT = 0.01666667;

export const onSpawn: OnEntitySpawn = (entityData: EntityData): EntityData => {};

export const update: EntityUpdate = (currentState: EntityState): EntityState => {
  const [lastX, lastY, lastZ] = currentState.position;
  const nextPosition: Vec3 = [lastX, lastY, lastZ - ENTITY_SPEED * DT];

  if (nextPosition[2] < 0) {
    // // Spawn one new entity
    // hy.spawnEntity(0, [nextPosition[0] + 2., nextPosition[1] + 1., 16]);

    // Reset the current entity
    nextPosition[2] = 32;

    // // Despawn any other entity so we don't flood the level

    // const entities = hy.getEntities();
    // const entityIds = Object.keys(entities);

    // // Despawn a random entity
    // let entityId = entityIds[Math.floor(Math.random() * entityIds.length)];
    // if (entityId !== undefined) {
    //   hy.despawnEntity(entityId);
    // }
  }

  return {
    ...currentState,
    position: nextPosition,
  };
};
