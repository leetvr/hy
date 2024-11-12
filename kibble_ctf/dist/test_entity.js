const ENTITY_SPEED = 5;
const DT = 0.01666667;
export const update = (currentState) => {
    const [lastX, lastY, lastZ] = currentState.position;
    const nextPosition = [lastX, lastY, lastZ - ENTITY_SPEED * DT];
    console.log("Entities", hy.getEntities());
    return Object.assign(Object.assign({}, currentState), { position: nextPosition });
};
