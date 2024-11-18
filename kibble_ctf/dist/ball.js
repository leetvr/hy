export const update = (id, entityState) => {
    // If we've travelled a long way, it's time to say goodbye
    if (Math.abs(entityState.position[0]) > 100 ||
        Math.abs(entityState.position[1]) > 100 ||
        Math.abs(entityState.position[2]) > 100) {
        hy.despawnEntity(id);
    }
    return entityState;
};
