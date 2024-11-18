import { EntityUpdate, OnEntitySpawn } from "../lib/hy";

export const update: EntityUpdate = (id, currentState, interactions) => {
  // NO-OP
  return currentState;
};

export const onSpawn: OnEntitySpawn = (entityData) => {
  if (entityData.model_path.match("red")) {
    return {
      ...entityData,
      name: "Red Flag",
    };
  }

  return {
    ...entityData,
    name: "Blue Flag",
  };
};
