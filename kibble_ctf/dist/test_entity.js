const ENTITY_SPEED = 15;
const DT = 0.01666667;
export const onSpawn = (entityData) => {
    if (Math.random() > 0.5) {
        return Object.assign(Object.assign({}, entityData), { model_path: "kibble_ctf/test_entity_alt.gltf" });
    }
    return entityData;
};
export const update = (currentState) => {
    const [lastX, lastY, lastZ] = currentState.position;
    const nextPosition = [lastX, lastY, lastZ - ENTITY_SPEED * DT];
    if (nextPosition[2] < 0) {
        // Reset the current entity
        nextPosition[2] = 32;
    }
    return Object.assign(Object.assign({}, currentState), { position: nextPosition });
};
