const MAX_RELOAD_TICKS = 30;
export const update = (id, currentState) => {
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
const moreBalls = ({ playerId, yaw, position }) => {
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
    const initialVelocity = [-Math.cos(angle) * speed, 0, Math.sin(angle) * speed];
    const initialPosition = [
        position[0] + -Math.cos(angle),
        position[1],
        position[2] + Math.sin(angle),
    ];
    hy.spawnEntity(2, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
