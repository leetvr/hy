import { EntityState, EntityUpdate, Interaction } from "../lib/hy";

const update: EntityUpdate = (
  id: string,
  entityState: EntityState,
  interactions: Interaction[],
): EntityState => {
  // If we've travelled a long way, it's time to say goodbye
  if (
    Math.abs(entityState.position[0]) > 100 ||
    Math.abs(entityState.position[1]) > 100 ||
    Math.abs(entityState.position[2]) > 100
  ) {
    hy.despawnEntity(id);
  }
  return entityState;
};
