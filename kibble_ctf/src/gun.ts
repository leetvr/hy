import type { EntityState, EntityUpdate, Vec3, Interaction } from "../lib/hy";

export const update: EntityUpdate = (
  id: string,
  currentState: EntityState,
): EntityState => {
  currentState.interactions.forEach(moreBalls);

  // Look, custom state!
  let currentCount = currentState.customState.counter;
  if (typeof currentCount !== "number") {
    currentCount = 0;
  } else {
  }

  currentState.customState.counter = currentCount + 1;

  return currentState;
};

// BALLS
const moreBalls = (interaction: Interaction) => {
  let speed = 50;

  // If the angle is wrong, don't find out why, just bash it into place
  let angle = interaction.facingAngle - Math.PI / 2;

  const initialVelocity: Vec3 = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
  const initialPosition: Vec3 = [
    interaction.position[0] + -Math.cos(angle),
    interaction.position[1],
    interaction.position[2] + Math.sin(angle),
  ];
  hy.spawnEntity(2, initialPosition, [0, 0, 0], initialVelocity);
};
