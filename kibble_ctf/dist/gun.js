export const update = (currentState, interactions) => {
    interactions.forEach(moreKanes);
    return currentState;
};
// more kanes
const moreKanes = (interaction) => {
    let speed = 10. + Math.random() * 25.;
    // If the angle is wrong, don't find out why, just bash it into place
    let angle = interaction.facingAngle - Math.PI / 2;
    hy.spawnEntity(0, interaction.position, [
        -Math.cos(angle) * speed,
        0.,
        Math.sin(angle) * speed
    ]);
};
