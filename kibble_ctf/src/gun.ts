import type { EntityState, EntityUpdate, Vec3, Interaction, Quat, OnEntitySpawn, EntityData } from "../lib/hy";

const DT = 0.01666667;

export const onSpawn: OnEntitySpawn = (entityData: EntityData): EntityData => {
  const { position, customState } = entityData.state;
  let newState = { ...customState };
  let newPosition: Vec3 = [...position];

  newState.timer = 0.;
  newState.spawnPosition = newPosition;
  newState.wasAnchored = false;
  if (entityData.state.anchor !== null) {
    newState.wasAnchored = true;
  }

  return {
    ...entityData,
    state: {
      ...entityData.state,
      position: newPosition,
      customState: newState
    }
  };
};

export const update: EntityUpdate = (id: string, currentState: EntityState): EntityState => {
  const { position, rotation, customState } = currentState;
  let newPosition: Vec3 = [...position];
  let newRotation: Quat = [...rotation];
  let newCustomState = { ...customState };

  if (currentState.anchor !== null) {
    newPosition = [0., 0., 0.];
    newRotation = [0., 0., 0., 1.];
    newCustomState.wasAnchored = true;
  } else {
    if (newCustomState.wasAnchored) {
      newCustomState.spawnPosition = newPosition;
      newCustomState.wasAnchored = false;
    }

    newCustomState.timer = (newCustomState.timer + DT) % 3.0;
    const t = newCustomState.timer / 3.0;
    newPosition[1] = newCustomState.spawnPosition[1] + Math.sin(t * 2 * Math.PI) * 0.15 + 0.75;

    // Artisanal hand rotated quaternion
    const angle = t * 2 * Math.PI;
    const sinHalfAngle = Math.sin(angle / 2);
    const cosHalfAngle = Math.cos(angle / 2);
    newRotation = [0, sinHalfAngle, 0, cosHalfAngle];
  }

  currentState.interactions.forEach(fireBullets);
  return {
    ...currentState,
    position: newPosition,
    rotation: newRotation,
    customState: newCustomState,
  }
};

// BULLETS
const fireBullets = ({ playerId, yaw, pitch, position }: Interaction) => {
  let speed = 50;
  const firingPlayerState = hy.getPlayerState(playerId);

  if (!firingPlayerState) {
    console.error(`We were shot by a non-existent player ${playerId}?`);
    return;
  }

  const team = firingPlayerState.customState.team;

  // If the angle is wrong, don't find out why, just bash it into place
  let fixedYaw = yaw + Math.PI / 2;
  // This one is just tuned to feel better so you don't constantly shoot the ground
  let fixedPitch = pitch + 0.2;

  let initialVelocity: Vec3 = [
    speed * Math.cos(fixedPitch) * Math.cos(fixedYaw),
    speed * Math.sin(fixedPitch),
    -(speed * Math.cos(fixedPitch) * Math.sin(fixedYaw))
  ];
  const initialPosition: Vec3 = [
    position[0] + (Math.cos(fixedPitch) * -Math.cos(fixedYaw)) * 0.25,
    position[1] + Math.sin(fixedPitch) * 0.25,
    position[2] + -(Math.cos(fixedPitch) * Math.sin(fixedYaw)) * 0.25,
  ];

  hy.spawnEntity(6, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
