export const update = (id, currentState, interactions) => {
    // NO-OP
    return currentState;
};
export const onSpawn = (entityData) => {
    if (entityData.model_path.match("red")) {
        return Object.assign(Object.assign({}, entityData), { name: "Red Flag" });
    }
    return Object.assign(Object.assign({}, entityData), { name: "Blue Flag" });
};
