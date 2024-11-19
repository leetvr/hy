import { EntityState, EntityUpdate, Interaction } from "../lib/hy";
const MAX_LIFETIME = 1000;
export const DT = 0.01666667; // 60HZ

export const update: EntityUpdate = (id: string, entityState: EntityState): EntityState => {
  // If we've travelled a long way, it's time to say goodbye
  if (
    Math.abs(entityState.position[0]) > 100 ||
    Math.abs(entityState.position[1]) > 100 ||
    Math.abs(entityState.position[2]) > 100
  ) {
    hy.despawnEntity(id);
    return entityState;
  }

  let lifetime = entityState.customState.lifetime;
  if (!lifetime) {
    lifetime = 0;
  }

  if (lifetime >= MAX_LIFETIME) {
    hy.despawnEntity(id);
    return entityState;
  }

  // Move on our own
  entityState.customState.lifetime = lifetime + 1;
  entityState.position[0] += entityState.velocity[0] * DT;
  entityState.position[1] += entityState.velocity[1] * DT;
  entityState.position[2] += entityState.velocity[2] * DT;

  hy.getCollisionsForEntity(id).forEach((collision) => {
    if (collision.collisionTarget === "entity" || collision.collisionTarget === "block") {
      hy.despawnEntity(id);
    }
  });

  return entityState;
};
