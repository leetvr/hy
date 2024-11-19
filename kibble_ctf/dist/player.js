const GRAVITY = -9.81; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 5.0; // Movement speed (units per second)
const JUMP_SPEED = 5.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const onSpawn = (playerID, currentState) => {
    const { customState, position } = currentState;
    let newCustomState = Object.assign({}, customState);
    newCustomState.health = MAX_HEALTH;
    newCustomState.spawnPosition = position;
    newCustomState.respawnTimer = 0;
    return Object.assign(Object.assign({}, currentState), { customState: newCustomState });
};
export const update = (playerID, currentState, controls) => {
    // Note(ll): I just put attachedEntities in currentState but mutating it in the script will not have any effect.
    // It's just a quick way to pass data to the script.
    const { position, velocity, animationState, isOnGround: wasOnGround, customState, attachedEntities, } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    let newAnimationState = animationState;
    let newCustomState = Object.assign({}, customState);
    let newControls = Object.assign({}, controls);
    let isAlive = newCustomState.health > 0;
    if (!isAlive) {
        if (attachedEntities["hand_left_anchor"]) {
            hy.detachEntity(attachedEntities["hand_left_anchor"][0], newPosition);
        }
        if (newCustomState.respawnTimer <= 0) {
            newPosition = newCustomState.spawnPosition;
            newCustomState.health = MAX_HEALTH;
            newCustomState.respawnTimer = 3.0;
        }
        newCustomState.respawnTimer -= DT;
        // Reset controls when player is dead
        newControls.move_direction = [0, 0];
        newControls.jump = false;
        newControls.fire = false;
    }
    const collisions = hy.getCollisionsForPlayer(playerID);
    collisions.forEach((collision) => {
        if (!isAlive) {
            return;
        }
        if (collision.collisionTarget == "entity") {
            let entityData = hy.getEntityData(collision.targetId);
            if (entityData != undefined) {
                const GUN_TYPE = 1;
                const BALL_TYPE = 2;
                const BLUE_FLAG_TYPE = 3;
                const RED_FLAG_TYPE = 4;
                const BULLET_TYPE = 6;
                if (entityData.entity_type == GUN_TYPE) {
                    // Pick up gun if there's nothing in the right hand
                    if (!attachedEntities["hand_right_anchor"]) {
                        hy.anchorEntity(collision.targetId, playerID, "hand_right_anchor");
                    }
                }
                if (entityData.entity_type == BULLET_TYPE) {
                    // Destroy bullet and take damage
                    hy.despawnEntity(collision.targetId);
                    hy.playSound("pain", currentState.position, 10);
                    newCustomState.health -= 1;
                    if (newCustomState.health <= 0) {
                        newCustomState.respawnTimer = RESPAWN_TIME;
                    }
                }
                if (entityData.entity_type == BALL_TYPE) {
                    // Destroy bullet and take damage
                    hy.despawnEntity(collision.targetId);
                    hy.playSound("pain", currentState.position, 10);
                    newCustomState.health -= 1; // TODO: More damage?
                    if (newCustomState.health <= 0) {
                        newCustomState.respawnTimer = RESPAWN_TIME;
                    }
                }
                if (entityData.entity_type == BLUE_FLAG_TYPE || entityData.entity_type == RED_FLAG_TYPE) {
                    // Don't do anything with a flag that is already carried
                    if (entityData.state.customState.carried) {
                        return;
                    }
                    let flag_team;
                    if (entityData.entity_type == BLUE_FLAG_TYPE) {
                        flag_team = "blue";
                    }
                    else {
                        flag_team = "red";
                    }
                    if (newCustomState.team == flag_team) {
                        // Interacting with a flag returns it to its spawn
                        hy.interactEntity(collision.targetId, playerID, position, newControls.camera_yaw);
                    }
                    else {
                        // Pick up the flag if we aren't already holding something in the left hand
                        if (!attachedEntities["hand_left_anchor"]) {
                            hy.anchorEntity(collision.targetId, playerID, "hand_left_anchor");
                        }
                    }
                }
            }
        }
    });
    if (newControls.fire) {
        let handItems = attachedEntities["hand_right_anchor"];
        if (handItems != undefined) {
            handItems.forEach((item) => {
                hy.interactEntity(item, playerID, position, newControls.camera_yaw);
            });
        }
    }
    // Handle horizontal movement
    const inputX = newControls.move_direction[0];
    const inputZ = newControls.move_direction[1];
    if (inputX !== 0 || inputZ !== 0) {
        // Normalize input direction
        const inputLength = Math.hypot(inputX, inputZ);
        const normalizedInput = [inputX / inputLength, inputZ / inputLength];
        // Rotate input by camera yaw to get world space direction
        const yaw = newControls.camera_yaw;
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
    if (isOnGround && newControls.jump) {
        newVelocity[1] = JUMP_SPEED;
        if (attachedEntities["hand_left_anchor"]) {
            hy.detachEntity(attachedEntities["hand_left_anchor"][0], newPosition);
        }
    }
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    if (!isAlive) {
        newAnimationState = "sleep";
    }
    return {
        position: newPosition,
        velocity: newVelocity,
        animationState: newAnimationState,
        customState: newCustomState,
        isOnGround,
        attachedEntities,
    };
};
const MAX_HEALTH = 5;
const RESPAWN_TIME = 3.0;
