import type { EntityState, EntityUpdate, Vec3, Interaction } from "../lib/hy";

export const update: EntityUpdate = (id: string, currentState: EntityState): EntityState => {
  currentState.interactions.forEach(fireBullets);
  return currentState;
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
  // This one is just adjusted to feel better so you don't constantly shoot the ground
  let fixedPitch = pitch + 0.2;

  let initialVelocity: Vec3 = [
    speed * Math.cos(fixedPitch) * Math.cos(fixedYaw),
    speed * Math.sin(fixedPitch),
    -(speed * Math.cos(fixedPitch) * Math.sin(fixedYaw))
  ];

  const initialPosition: Vec3 = [
    position[0] + Math.cos(fixedPitch) * -Math.cos(fixedYaw),
    position[1] + Math.sin(fixedPitch) + 0.5,
    position[2] + -(Math.cos(fixedPitch) * Math.sin(fixedYaw)),
  ];

  hy.spawnEntity(6, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
