const DT = 0.01666667;
export const onSpawn = (entityData) => {
    const { position, customState } = entityData.state;
    let newState = Object.assign({}, customState);
    let newPosition = [...position];
    newState.timer = 0.;
    newState.spawnPosition = newPosition;
    newState.wasAnchored = false;
    if (entityData.state.anchor !== null) {
        newState.wasAnchored = true;
    }
    return Object.assign(Object.assign({}, entityData), { state: Object.assign(Object.assign({}, entityData.state), { position: newPosition, customState: newState }) });
};
export const update = (id, currentState) => {
    const { position, rotation, customState } = currentState;
    let newPosition = [...position];
    let newRotation = [...rotation];
    let newCustomState = Object.assign({}, customState);
    if (currentState.anchor !== null) {
        newPosition = [0., 0., 0.];
        newRotation = [0., 0., 0., 1.];
        newCustomState.wasAnchored = true;
    }
    else {
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
    }
    currentState.interactions.forEach(fireBullets);
    return Object.assign(Object.assign({}, currentState), { position: newPosition, rotation: newRotation, customState: newCustomState });
};
// BULLETS
const fireBullets = ({ playerId, yaw, pitch, position }) => {
    let speed = 50;
    const firingPlayerState = hy.getPlayerState(playerId);
    if (!firingPlayerState) {
        console.error(`We were shot by a non-existent player ${playerId}?`);
        return;
    }
    const team = firingPlayerState.customState.team;
    // If the angle is wrong, don't find out why, just bash it into place
    let fixedYaw = yaw + Math.PI / 2;
    // This one is just tuned to feel better so you don't constantly shoot the ground
    let fixedPitch = pitch + 0.3;
    let initialVelocity = [
        speed * Math.cos(fixedPitch) * Math.cos(fixedYaw),
        speed * Math.sin(fixedPitch),
        -(speed * Math.cos(fixedPitch) * Math.sin(fixedYaw))
    ];
    const initialPosition = [
        position[0] + (Math.cos(fixedPitch) * -Math.cos(fixedYaw)) * 0.25,
        position[1] + 0.25 + Math.sin(fixedPitch) * 0.25,
        position[2] + -(Math.cos(fixedPitch) * Math.sin(fixedYaw)) * 0.25,
    ];
    hy.spawnEntity(6, initialPosition, [0, 0, 0], initialVelocity, { firedByTeam: team });
};
