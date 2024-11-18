const GRAVITY = -9.81; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 5.0; // Movement speed (units per second)
const JUMP_SPEED = 5.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const update = (playerID, currentState, controls) => {
    // Note(ll): I just put attachedEntities in currentState but mutating it in the script will not have any effect.
    // It's just a quick way to pass data to the script.
    const { position, velocity, animationState, isOnGround: wasOnGround, customState, attachedEntities } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    let newAnimationState = animationState;
    if (controls.fire) {
        let handItems = attachedEntities["hand_right_anchor"];
        let gun;
        if (handItems == undefined || handItems.length == 0) {
            gun = hy.spawnEntity(1, [0, -0.5, -0.5], [0, 0, 0], [0, 0, 0]);
            hy.anchorEntity(gun, playerID, "hand_right_anchor");
            handItems = [gun];
        }
        handItems.forEach((item) => {
            hy.interactEntity(item, playerID, position, controls.camera_yaw);
        });
    }
    const collisions = hy.getCollisionsForPlayer(playerID);
    collisions.forEach((collision) => {
        if (collision.collisionTarget == "entity") {
            let entityData = hy.getEntityData(collision.targetId);
            if (entityData != undefined) {
                const GUN_TYPE = 1;
                const BULLET_TYPE = 2;
                const BLUE_FLAG_TYPE = 3;
                const RED_FLAG_TYPE = 4;
                if (entityData.entity_type == GUN_TYPE) {
                    // Pick up gun if there's nothing in the right hand
                    if (!attachedEntities["hand_right_anchor"]) {
                        hy.anchorEntity(collision.targetId, playerID, "hand_right_anchor");
                    }
                }
                if (entityData.entity_type == BULLET_TYPE) {
                    // Destroy bullet
                    hy.despawnEntity(collision.targetId);
                }
                if (entityData.entity_type == BLUE_FLAG_TYPE || entityData.entity_type == RED_FLAG_TYPE) {
                    // Capture the flag
                    if (!attachedEntities["hand_left_anchor"]) {
                        hy.anchorEntity(collision.targetId, playerID, "hand_left_anchor");
                    }
                }
            }
        }
    });
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
        // TODO: Apply damping to horizontal velocity when no input
        newVelocity[0] *= 0.7;
        newVelocity[2] *= 0.7;
        newAnimationState = "idle";
    }
    // Apply gravity
    if (!wasOnGround) {
        newVelocity[1] += GRAVITY * DT;
    }
    // Update position based on velocity and delta time
    const desiredMovement = [newVelocity[0], newVelocity[1], newVelocity[2]];
    const { correctedMovement, isOnGround } = hy.checkMovementForCollisions(playerID, position, desiredMovement);
    newVelocity[0] = correctedMovement[0];
    newVelocity[1] = correctedMovement[1];
    newVelocity[2] = correctedMovement[2];
    if (isOnGround && controls.jump) {
        newVelocity[1] = JUMP_SPEED;
        if (attachedEntities["hand_left_anchor"]) {
            hy.detachEntity(attachedEntities["hand_left_anchor"][0], newPosition);
        }
    }
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    // Look, custom state!
    let currentCount = customState.counter;
    if (typeof currentCount !== "number") {
        currentCount = 0;
    }
    customState.counter = currentCount + 1;
    return {
        position: newPosition,
        velocity: newVelocity,
        animationState: newAnimationState,
        isOnGround,
        customState,
        attachedEntities,
    };
};
