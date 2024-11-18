export const update = (id, currentState, interactions) => {
    interactions.forEach(moreBalls);
    return currentState;
};
// BALLS
const moreBalls = (interaction) => {
    let speed = 50;
    // If the angle is wrong, don't find out why, just bash it into place
    let angle = interaction.facingAngle - Math.PI / 2;
    const initialPosition = [
        interaction.position[0] + 0.2,
        interaction.position[1],
        interaction.position[2] + 0.2,
    ];
    hy.spawnEntity(2, initialPosition, [0, 0, 0], [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed]);
};
