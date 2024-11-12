const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 10.0; // Movement speed (units per second)
const JUMP_SPEED = 12.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
const PLAYER_SIZE = [0.5, 1.5, 0.5]; // Player size (x, y, z)
export const update = (currentState, controls, collisions) => {
    const { position, velocity } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    // Handle horizontal movement
    const inputX = controls.move_direction[0];
    const inputZ = controls.move_direction[1];
    if (inputX !== 0 || inputZ !== 0) {
        // Normalize input direction
        const inputLength = Math.hypot(inputX, inputZ);
        const normalizedInput = [inputX / inputLength, inputZ / inputLength];
        // Rotate input by camera yaw to get world space direction
        const yaw = controls.camera_yaw;
        const sinYaw = Math.sin(yaw);
        const cosYaw = Math.cos(yaw);
        // Compute movement direction in world space
        const moveDirX = normalizedInput[0] * cosYaw - normalizedInput[1] * sinYaw;
        const moveDirZ = normalizedInput[0] * sinYaw + normalizedInput[1] * cosYaw;
        // Update horizontal velocity
        newVelocity[0] = moveDirX * MOVE_SPEED;
        newVelocity[2] = -moveDirZ * MOVE_SPEED;
    }
    else {
        // Apply damping to horizontal velocity when no input
        newVelocity[0] *= 0.9; // Adjust damping factor as needed
        newVelocity[2] *= 0.9;
    }
    // Handle jumping
    if (controls.jump && position[1] <= 1.01) {
        // Simple ground check
        newVelocity[1] = JUMP_SPEED;
    }
    // Apply gravity to vertical velocity
    newVelocity[1] += GRAVITY * DT;
    // Simple collision resolution
    // collisions = collisions.filter((collision) => {
    //   // return length(collision.resolution) < 0.5;
    // });
    collisions.sort((a, b) => length(a.resolution) - length(b.resolution));
    let resolutionsSum = [0, 0, 0];
    if (collisions.length > 0) {
        console.log("did collide");
        const { block, resolution, normal } = collisions[0];
        // Move player out of block
        for (let i = 0; i < 3; i++) {
            if (resolutionsSum[i] == 0 && resolution[i] != 0) {
                newPosition[i] += resolution[i];
                resolutionsSum[i] += resolution[i];
                if (newVelocity[i] * normal[i] < 0) {
                    newVelocity[i] = 0.;
                }
            }
        }
    }
    // Update position based on velocity and delta time
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    return {
        position: newPosition,
        velocity: newVelocity,
    };
};
function length(v) {
    return Math.hypot(v[0], v[1], v[2]);
}
