export const update = (id, currentState) => {
    if (currentState.anchor != null) {
        // If the flag is attached to a player, move it with the player
        currentState.position = [0, 0.25, 0];
        // This quaternion brought to you by ChatGPT
        // Why have usable APIs when you have math robuts
        currentState.rotation = [-0.7071068, 0, 0, 0.7071068];
    }
    else {
        // Reset the rotation when the flag is no longer held
        currentState.rotation = [0, 0, 0, 1];
    }
    return currentState;
};
export const onSpawn = (entityData) => {
    if (entityData.model_path.match("red")) {
        return Object.assign(Object.assign({}, entityData), { name: "Red Flag" });
    }
    return Object.assign(Object.assign({}, entityData), { name: "Blue Flag" });
};
