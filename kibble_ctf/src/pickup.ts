import type {
    EntityState,
    EntityUpdate,
    OnEntitySpawn,
    EntityData,
    Vec3,
    Quat,
} from "../lib/hy";

const DT = 0.01666667;

export const onSpawn: OnEntitySpawn = (entityData: EntityData): EntityData => {
    const { position, customState } = entityData.state;
    let newState = { ...customState };
    let newPosition: Vec3 = [...position];

    newState.timer = 0.;
    newState.spawnPosition = newPosition;

    return {
        ...entityData,
        state: {
            ...entityData.state,
            position: newPosition,
            customState: newState
        }
    };
};

export const update: EntityUpdate = (
    id: string,
    currentState: EntityState,
): EntityState => {
    const { position, rotation, customState } = currentState;
    let newPosition: Vec3 = [...position];
    let newRotation: Quat = [...rotation];
    let newCustomState = { ...customState };

    newCustomState.timer = (newCustomState.timer + DT) % 3.0;
    const t = newCustomState.timer / 3.0;
    newPosition[1] = newCustomState.spawnPosition[1] + Math.sin(t * 2 * Math.PI) * 0.15 + 0.75;

    // Artisanal hand rotated quaternion
    const angle = t * 2 * Math.PI;
    const sinHalfAngle = Math.sin(angle / 2);
    const cosHalfAngle = Math.cos(angle / 2);
    newRotation = [0, sinHalfAngle, 0, cosHalfAngle];

    return {
        ...currentState,
        position: newPosition,
        rotation: newRotation,
        customState: newCustomState
    };
};
