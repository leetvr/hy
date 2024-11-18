const DT = 0.01666667;
export const onSpawn = (entityData) => {
    if (Math.random() > 0.5) {
        return Object.assign(Object.assign({}, entityData), { model_path: "kibble_ctf/test_entity_alt.gltf" });
    }
    return entityData;
};
export const update = (id, currentState, interactions) => {
    const [lastX, lastY, lastZ] = currentState.position;
    const [velX, velY, velZ] = currentState.velocity;
    const nextPosition = [lastX + velX * DT, lastY + velY * DT, lastZ + velZ * DT];
    if (nextPosition[0] > 32) {
        nextPosition[0] = 0;
    }
    if (nextPosition[0] < 0) {
        nextPosition[0] = 32;
    }
    if (nextPosition[2] > 32) {
        nextPosition[2] = 0;
    }
    if (nextPosition[2] < 0) {
        nextPosition[2] = 32;
    }
    return Object.assign(Object.assign({}, currentState), { position: nextPosition });
};
