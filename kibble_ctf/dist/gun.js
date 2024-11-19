export const update = (id, currentState) => {
    currentState.interactions.forEach(fireBullets);
    return currentState;
};
// BULLETS
const fireBullets = (interaction) => {
    let speed = 50;
    // If the angle is wrong, don't find out why, just bash it into place
    let angle = interaction.facingAngle - Math.PI / 2;
    const initialVelocity = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
    const initialPosition = [
        interaction.position[0] + -Math.cos(angle),
        interaction.position[1] + 0.5,
        interaction.position[2] + Math.sin(angle),
    ];
    hy.spawnEntity(6, initialPosition, [0, 0, 0], initialVelocity);
};
