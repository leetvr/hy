const GRAVITY = -9.81; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 5.0; // Movement speed (units per second)
const JUMP_SPEED = 5.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const onAddPlayer = (worldState, playerID, currentState) => {
    // Give this man a gun
    let gun = hy.spawnEntity(1, [0, -0.5, -0.5], [0, 0, 0], [0, 0, 0]);
    hy.anchorEntity(gun, playerID, "hand_right_anchor");
    return currentState;
};
