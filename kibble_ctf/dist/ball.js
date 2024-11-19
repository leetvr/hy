const MAX_LIFETIME = 1000;
export const update = (id, entityState) => {
    // If we've travelled a long way, it's time to say goodbye
    if (Math.abs(entityState.position[0]) > 100 ||
        Math.abs(entityState.position[1]) > 100 ||
        Math.abs(entityState.position[2]) > 100) {
        hy.despawnEntity(id);
        return entityState;
    }
    let lifetime = entityState.customState.lifetime;
    if (!lifetime) {
        lifetime = 0;
    }
    if (lifetime >= MAX_LIFETIME) {
        hy.despawnEntity(id);
        return entityState;
    }
    entityState.customState.lifetime = lifetime + 1;
    return entityState;
};
