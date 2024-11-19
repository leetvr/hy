import type { EntityState, EntityUpdate, Vec3, Interaction } from "../lib/hy";

const MAX_RELOAD_TICKS = 30;
export const update: EntityUpdate = (id: string, currentState: EntityState): EntityState => {
  let reloadTime = currentState.customState.reloadTime;
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

  currentState.customState.reloadTime = reloadTime - 1;
  return currentState;
};

// BALLS
const moreBalls = (interaction: Interaction) => {
  let speed = 50;

  // If the angle is wrong, don't find out why, just bash it into place
  let angle = interaction.facingAngle - Math.PI / 2;

  // TODO: fire multiple balls? maths too hard for kane brain
  const initialVelocity: Vec3 = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
  const initialPosition: Vec3 = [
    interaction.position[0] + -Math.cos(angle),
    interaction.position[1],
    interaction.position[2] + Math.sin(angle),
  ];
  hy.spawnEntity(2, initialPosition, [0, 0, 0], initialVelocity);
};
