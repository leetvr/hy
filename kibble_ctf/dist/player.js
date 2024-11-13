const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 10.0; // Movement speed (units per second)
const JUMP_SPEED = 12.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
const PLAYER_SIZE = [0.5, 1.5, 0.5]; // Player size (x, y, z)
export const update = (playerID, currentState, controls, collisions) => {
    const { position, velocity, animationState } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    let newAnimationState = animationState;
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
        newAnimationState = "run";
    }
    else {
        // Apply damping to horizontal velocity when no input
        newVelocity[0] *= 0.7; // Adjust damping factor as needed
        newVelocity[2] *= 0.7;
        newAnimationState = "idle";
    }
    // Ground detection
    const isOnGround = hy.isPlayerOnGround(playerID);
    // Handle jumping, falling
    if (!isOnGround) {
        newVelocity[1] += GRAVITY * DT;
    }
    else if (controls.jump) {
        newVelocity[1] += JUMP_SPEED * DT;
    }
    else {
        newVelocity[1] = 0;
    }
    // Apply gravity to vertical velocity
    // Update position based on velocity and delta time
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    return {
        position: newPosition,
        velocity: newVelocity,
        animationState: newAnimationState,
    };
};
function length(v) {
    return Math.hypot(v[0], v[1], v[2]);
}
