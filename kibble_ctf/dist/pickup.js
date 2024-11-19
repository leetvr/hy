const DT = 0.01666667;
export const onSpawn = (entityData) => {
    const { position, customState } = entityData.state;
    let newState = Object.assign({}, customState);
    let newPosition = [...position];
    newState.timer = 0.;
    newState.spawnPosition = newPosition;
    return Object.assign(Object.assign({}, entityData), { state: Object.assign(Object.assign({}, entityData.state), { position: newPosition, customState: newState }) });
};
export const update = (id, currentState) => {
    const { position, rotation, customState } = currentState;
    let newPosition = [...position];
    let newRotation = [...rotation];
    let newCustomState = Object.assign({}, customState);
    newCustomState.timer = (newCustomState.timer + DT) % 3.0;
    const t = newCustomState.timer / 3.0;
    newPosition[1] = newCustomState.spawnPosition[1] + Math.sin(t * 2 * Math.PI) * 0.15 + 0.75;
    // Artisanal hand rotated quaternion
    const angle = t * 2 * Math.PI;
    const sinHalfAngle = Math.sin(angle / 2);
    const cosHalfAngle = Math.cos(angle / 2);
    newRotation = [0, sinHalfAngle, 0, cosHalfAngle];
    return Object.assign(Object.assign({}, currentState), { position: newPosition, rotation: newRotation, customState: newCustomState });
};
