export const onSpawn = (entityData) => {
    let customState = entityData.state.customState;
    customState.spawnPosition = entityData.state.position;
    customState.carried = false;
    if (entityData.model_path.match("red")) {
        return Object.assign(Object.assign({}, entityData), { name: "Red Flag", state: Object.assign(Object.assign({}, entityData.state), { customState: customState }) });
    }
    return Object.assign(Object.assign({}, entityData), { name: "Blue Flag", state: Object.assign(Object.assign({}, entityData.state), { customState: customState }) });
};
export const update = (id, currentState) => {
    const { anchor, customState, position, rotation } = currentState;
    let newPosition = [...position];
    let newRotation = [...rotation];
    let newCustomState = Object.assign({}, customState);
    if (currentState.anchor != null) {
        // If the flag is attached to a player, move it with the player
        newPosition = [0, 0.25, 0];
        // This quaternion brought to you by ChatGPT
        // Why have usable APIs when you have math robuts
        newRotation = [-0.7071068, 0, 0, 0.7071068];
        newCustomState.carried = true;
    }
    else {
        // Reset the rotation when the flag is no longer held
        newRotation = [0, 0, 0, 1];
        newCustomState.carried = false;
    }
    // On interaction, return the flag to its spawn position
    if (currentState.interactions.length > 0) {
        newPosition = customState.spawnPosition;
        hy.detachEntity(id, customState.spawnPosition);
    }
    return Object.assign(Object.assign({}, currentState), { position: newPosition, rotation: newRotation, customState: newCustomState });
};
