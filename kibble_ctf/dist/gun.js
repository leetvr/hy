export const update = (id, currentState, interactions) => {
    interactions.forEach(moreBalls);
    // Look, custom state!
    let currentCount = currentState.customState.counter;
    if (typeof currentCount !== "number") {
        currentCount = 0;
    }
    else {
    }
    currentState.customState.counter = currentCount + 1;
    return currentState;
};
// BALLS
const moreBalls = (interaction) => {
    let speed = 50;
    // If the angle is wrong, don't find out why, just bash it into place
    let angle = interaction.facingAngle - Math.PI / 2;
    const initialVelocity = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
    const initialPosition = [
        interaction.position[0] + -Math.cos(angle),
        interaction.position[1],
        interaction.position[2] + Math.sin(angle),
    ];
    hy.spawnEntity(2, initialPosition, [0, 0, 0], initialVelocity);
};
