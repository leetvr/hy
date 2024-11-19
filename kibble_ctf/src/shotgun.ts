import type { EntityState, EntityUpdate, Vec3, Interaction, OnEntitySpawn, EntityData, Quat } from "../lib/hy";

const MAX_RELOAD_TICKS = 30;
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
  const { position, rotation, scale, customState } = currentState;
  let newPosition: Vec3 = [...position];
  let newRotation: Quat = [...rotation];
  let newScale: Vec3 = [...scale];
  let newCustomState = { ...customState };


  if (currentState.anchor != null) {
    newPosition = [0., 0., 0.];
    newRotation = [0., 0., 0., 1.];
    newScale = [1., 1., 1.];
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

    newScale = [0.5, 0.5, 0.5];
  }

  let reloadTime = newCustomState.reloadTime;
  if (typeof reloadTime !== "number") {
    reloadTime = 0;
  }

  currentState.interactions.forEach((interaction) => {
    if (reloadTime > 0) {
      return;
    }

    moreBalls(interaction);
    reloadTime = MAX_RELOAD_TICKS;
  });

  newCustomState.reloadTime = reloadTime - 1;
  return {
    ...currentState,
    position: newPosition,
    rotation: newRotation,
    scale: newScale,
    customState: newCustomState,
  }
};

// BALLS
const moreBalls = ({ playerId, yaw, position }: Interaction) => {
  let speed = 50;
  const firingPlayerState = hy.getPlayerState(playerId);

  if (!firingPlayerState) {
    console.error(`We were shot by a non-existent player ${playerId}?`);
    return;
  }

  const team = firingPlayerState.customState.team;

  // If the angle is wrong, don't find out why, just bash it into place
  let angle = yaw - Math.PI / 2;

  // TODO: fire multiple balls? maths too hard for kane brain
  const initialVelocity: Vec3 = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
  const initialPosition: Vec3 = [
    position[0] + -Math.cos(angle),
    position[1],
    position[2] + Math.sin(angle),
  ];
  hy.spawnEntity(2, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
