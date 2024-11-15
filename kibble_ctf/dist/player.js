const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 8.0; // Movement speed (units per second)
const JUMP_SPEED = 12.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const update = (playerID, currentState, controls, collisions) => {
    const { position, velocity, animationState } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    let newAnimationState = animationState;
    if (controls.fire) {
        let gun = hy.spawnEntity(1, [0, 0., -0.5], [0, 0, 0], [0, 0, 0]);
        hy.anchorEntity(gun, playerID, "hand_right_anchor");
        hy.interactEntity(gun, playerID, position, controls.camera_yaw);
    }
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
        newVelocity[1] = JUMP_SPEED;
    }
    else {
        newVelocity[1] = 0;
    }
    // Update position based on velocity and delta time
    const movement = [newVelocity[0] * DT, newVelocity[1] * DT, newVelocity[2] * DT];
    newPosition[0] += movement[0];
    newPosition[1] += movement[1];
    newPosition[2] += movement[2];
    const adjustedMovement = hy.checkMovementForCollisions(playerID, movement);
    // Check for collisions with blocks
    if (adjustedMovement) {
        newPosition[0] += adjustedMovement[0];
        newPosition[1] += adjustedMovement[1];
        newPosition[2] += adjustedMovement[2];
        return {
            position: newPosition,
            velocity: newVelocity,
            animationState: newAnimationState,
        };
    }
    else {
        return {
            position: newPosition,
            velocity: newVelocity,
            animationState: newAnimationState,
        };
    }
};
