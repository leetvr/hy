import { EntityUpdate, OnEntitySpawn } from "../lib/hy";

export const update: EntityUpdate = (id, currentState) => {
  if (currentState.anchor != null) {
    // If the flag is attached to a player, move it with the player
    currentState.position = [0, 0.25, 0];

    // This quaternion brought to you by ChatGPT
    // Why have usable APIs when you have math robuts
    currentState.rotation = [-0.7071068, 0, 0, 0.7071068];
  } else {
    // Reset the rotation when the flag is no longer held
    currentState.rotation = [0, 0, 0, 1];
  }

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
