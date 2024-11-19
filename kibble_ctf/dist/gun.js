export const update = (id, currentState) => {
    currentState.interactions.forEach(fireBullets);
    return currentState;
};
// BULLETS
const fireBullets = ({ playerId, facingAngle, position }) => {
    let speed = 50;
    const firingPlayerState = hy.getPlayerState(playerId);
    if (!firingPlayerState) {
        console.error(`We were shot by a non-existent player ${playerId}?`);
        return;
    }
    const team = firingPlayerState.customState.team;
    // If the angle is wrong, don't find out why, just bash it into place
    let angle = facingAngle - Math.PI / 2;
    const initialVelocity = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
    const initialPosition = [
        position[0] + -Math.cos(angle),
        position[1] + 0.5,
        position[2] + Math.sin(angle),
    ];
    hy.spawnEntity(6, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
