import { EntityState, EntityUpdate, Vec3 } from "../lib/hy";

const ENTITY_SPEED = 5;
const DT = 0.01666667;

export const update: EntityUpdate = (currentState: EntityState): EntityState => {
  const [lastX, lastY, lastZ] = currentState.position;
  const nextPosition: Vec3 = [lastX, lastY, lastZ - ENTITY_SPEED * DT];
  return {
    ...currentState,
    position: nextPosition,
  };
};
