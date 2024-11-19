import { CustomState, EntityData, EntityState, EntityUpdate, OnEntitySpawn, Quat, Vec3 } from "../lib/hy";


export const onSpawn: OnEntitySpawn = (entityData: EntityData) => {
  let customState = entityData.state.customState;

  customState.spawnPosition = entityData.state.position;
  customState.carried = false;

  if (entityData.model_path.match("red")) {
    return {
      ...entityData,
      name: "Red Flag",
      state: {
        ...entityData.state,
        customState: customState
      }
    };
  }

  return {
    ...entityData,
    name: "Blue Flag",
    state: {
      ...entityData.state,
      customState: customState
    }
  };
};


export const update: EntityUpdate = (id, currentState: EntityState) => {
  const { anchor, customState, position, rotation, scale } = currentState;
  let newPosition: Vec3 = [...position];
  let newRotation: Quat = [...rotation];
  let newScale: Vec3 = [...scale];
  let newCustomState: CustomState = { ...customState };

  if (anchor != null) {
    // If the flag is attached to a player, move it with the player
    newPosition = [0, 0, -0.25];

    // This quaternion brought to you by ChatGPT
    // Why have usable APIs when you have math robuts
    newRotation = [0.2588190451, 0., 0., 0.9659258263];

    newScale = [2., 3., 2.];

    newCustomState.carried = true;
  } else {
    // Reset the rotation when the flag is no longer held
    newRotation = [0, 0, 0, 1];
    newScale = [1., 1., 1.];

    newCustomState.carried = false;
  }

  // On interaction, return the flag to its spawn position
  if (currentState.interactions.length > 0) {
    newPosition = customState.spawnPosition;
    hy.detachEntity(id, customState.spawnPosition);
  }

  return {
    ...currentState,
    position: newPosition,
    rotation: newRotation,
    scale: newScale,
    customState: newCustomState,
  };
}