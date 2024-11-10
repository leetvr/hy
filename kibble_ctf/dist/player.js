const GRAVITY = -9.81; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 10.0; // Movement speed (units per second)
const JUMP_SPEED = 5.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const update = (currentState, controls) => {
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
        console.log("Yaw", yaw, "normalizedInput", normalizedInput, "moveDirX", moveDirX, "moveDirZ", moveDirZ);
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
    // Update position based on velocity and delta time
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    console.log("newPosition", newPosition, "newVelocity", newVelocity);
    // Simple collision detection with the ground
    if (newPosition[1] < 1) {
        newPosition[1] = 1; // Make sure we're on the ground
        newVelocity[1] = 0; // Make sure we're no longer falling
    }
    return {
        position: newPosition,
        velocity: newVelocity,
    };
};
