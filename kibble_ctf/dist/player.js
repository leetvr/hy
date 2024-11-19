const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 7.0; // Movement speed (units per second)
const JUMP_SPEED = 8.0; // Jump initial velocity (units per second)
const MIN_FALL_SPEED = -20.;
const DT = 1 / 60; // Fixed delta time (seconds per frame)
export const onSpawn = (playerId, currentState) => {
    const { customState, position } = currentState;
    let newCustomState = Object.assign({}, customState);
    let newModelPath;
    if (customState.team == "red") {
        newModelPath = "kibble_ctf/player_red.gltf";
    }
    else {
        newModelPath = "kibble_ctf/player_blue.gltf";
    }
    newCustomState.maxHealth = MAX_HEALTH;
    newCustomState.health = MAX_HEALTH;
    newCustomState.spawnPosition = position;
    newCustomState.respawnTimer = RESPAWN_TIME;
    newCustomState.coyoteTime = 0.;
    newCustomState.jumpInputTime = 0.;
    let gun = hy.spawnEntity(GUN_TYPE_ID, [0, 0, 0], [0, 0, 0], [0, 0, 0]);
    hy.anchorEntity(gun, playerId, "hand_right_anchor");
    newCustomState.ammo = max_ammo(GUN_TYPE_ID);
    newCustomState.maxAmmo = max_ammo(GUN_TYPE_ID);
    // NOTE(ll): modelPath *must* be set here, otherwise the model won't be loaded.
    return Object.assign(Object.assign({}, currentState), { customState: newCustomState, modelPath: newModelPath });
};
export const update = (playerID, currentState, controls) => {
    // Note(ll): I just put attachedEntities in currentState but mutating it in the script will not have any effect.
    // It's just a quick way to pass data to the script.
    const { position, velocity, animationState, facingAngle, isOnGround: wasOnGround, customState, attachedEntities, } = currentState;
    let newPosition = [...position];
    let newVelocity = [...velocity];
    let newFacingAngle = facingAngle;
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
                if (entityData.entity_type == GUN_TYPE_ID) {
                    // Pick up gun if there's nothing in the right hand
                    if (!attachedEntities["hand_right_anchor"]) {
                        hy.anchorEntity(collision.targetId, playerID, "hand_right_anchor");
                    }
                }
                if (entityData.entity_type == AMMO_TYPE_ID) {
                    if (newCustomState.ammo < newCustomState.maxAmmo) {
                        hy.despawnEntity(collision.targetId);
                        newCustomState.ammo = newCustomState.maxAmmo;
                    }
                }
                if (entityData.entity_type == BULLET_TYPE_ID) {
                    // No friendly fire!
                    const firedByTeam = entityData.state.customState.firedByTeam;
                    if (firedByTeam == customState.team) {
                        return;
                    }
                    // Destroy bullet and take damage
                    hy.despawnEntity(collision.targetId);
                    hy.playSound("pain", currentState.position, 10);
                    newCustomState.health -= 1;
                    if (newCustomState.health <= 0) {
                        newCustomState.respawnTimer = RESPAWN_TIME;
                    }
                }
                if (entityData.entity_type == BALL_TYPE_ID) {
                    const entityData = hy.getEntityData(collision.targetId);
                    // No friendly fire!
                    const firedByTeam = entityData.state.customState.firedByTeam;
                    if (firedByTeam == customState.team) {
                        return;
                    }
                    // Destroy bullet and take damage
                    hy.despawnEntity(collision.targetId);
                    hy.playSound("pain", currentState.position, 10);
                    newCustomState.health -= 1; // TODO: More damage?
                    if (newCustomState.health <= 0) {
                        newCustomState.respawnTimer = RESPAWN_TIME;
                    }
                }
                if (entityData.entity_type == BLUE_FLAG_TYPE_ID || entityData.entity_type == RED_FLAG_TYPE_ID) {
                    // Don't do anything with a flag that is already carried
                    if (entityData.state.customState.carried) {
                        return;
                    }
                    let flag_team;
                    if (entityData.entity_type == BLUE_FLAG_TYPE_ID) {
                        flag_team = "blue";
                    }
                    else {
                        flag_team = "red";
                    }
                    if (newCustomState.team == flag_team) {
                        // Interacting with a flag returns it to its spawn
                        hy.interactEntity(collision.targetId, playerID, position, newControls.camera_yaw, newControls.camera_pitch);
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
    let isFiring = false;
    if (newControls.fire) {
        newFacingAngle = controls.camera_yaw;
        let handItems = attachedEntities["hand_right_anchor"];
        if (handItems != undefined) {
            handItems.forEach((item) => {
                if (newCustomState.ammo > 0) {
                    isFiring = true;
                    hy.interactEntity(item, playerID, position, newControls.camera_yaw, newControls.camera_pitch);
                    newCustomState.ammo -= 1;
                }
            });
        }
    }
    // Handle horizontal movement
    const inputX = newControls.move_direction[0];
    const inputZ = newControls.move_direction[1];
    if (inputX !== 0 || inputZ !== 0) {
        newFacingAngle = controls.camera_yaw;
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
    }
    else {
        // TODO: Apply damping to horizontal velocity when no input
        newVelocity[0] *= 0.7;
        newVelocity[2] *= 0.7;
    }
    // Apply gravity
    newVelocity[1] += GRAVITY * DT;
    // Update position based on velocity and delta time
    const desiredMovement = [newVelocity[0], newVelocity[1], newVelocity[2]];
    const { correctedMovement, isOnGround } = hy.checkMovementForCollisions(playerID, position, desiredMovement);
    newVelocity[0] = correctedMovement[0];
    newVelocity[1] = correctedMovement[1];
    newVelocity[2] = correctedMovement[2];
    newVelocity[1] = Math.max(newVelocity[1], MIN_FALL_SPEED);
    if (isOnGround) {
        newCustomState.coyoteTime = COYOTE_TIME;
    }
    else {
        newCustomState.coyoteTime = Math.max(0., newCustomState.coyoteTime - DT);
    }
    if (controls.jump) {
        newCustomState.jumpInputTime = JUMP_INPUT_TIME;
    }
    else {
        newCustomState.jumpInputTime = Math.max(0., newCustomState.jumpInputTime - DT);
    }
    let didJump = false;
    if (newCustomState.coyoteTime > 0. && newCustomState.jumpInputTime > 0.) {
        didJump = true;
        newVelocity[1] = JUMP_SPEED;
        newCustomState.coyoteTime = 0.;
    }
    newPosition[0] += newVelocity[0] * DT;
    newPosition[1] += newVelocity[1] * DT;
    newPosition[2] += newVelocity[2] * DT;
    if (!isAlive) {
        newAnimationState = "sleep";
    }
    else if (isFiring) {
        newAnimationState = "simple_interact";
    }
    else if (didJump) {
        newAnimationState = "jump_loop";
    }
    else if ((inputX !== 0 || inputZ !== 0) && isOnGround) {
        newAnimationState = "run";
    }
    else if (isOnGround) {
        newAnimationState = "idle";
    }
    // Players die when they are below the map
    if (newPosition[1] < -10) {
        newCustomState.health = 0;
    }
    return Object.assign(Object.assign({}, currentState), { position: newPosition, velocity: newVelocity, facingAngle: newFacingAngle, animationState: newAnimationState, customState: newCustomState, isOnGround,
        attachedEntities });
};
const GUN_TYPE_ID = 1;
const BALL_TYPE_ID = 2;
const BLUE_FLAG_TYPE_ID = 3;
const RED_FLAG_TYPE_ID = 4;
const SHOTGUN_TYPE_ID = 5;
const BULLET_TYPE_ID = 6;
const AMMO_TYPE_ID = 7;
function max_ammo(entity_type) {
    if (entity_type == GUN_TYPE_ID) {
        return 10;
    }
    else if (entity_type == SHOTGUN_TYPE_ID) {
        return 3;
    }
    else {
        return 0;
    }
}
const MAX_HEALTH = 5;
const RESPAWN_TIME = 3.0;
// Coyote time is the time after the player has left the ground during which they can still jump
const COYOTE_TIME = 0.1;
// The time after the player has pressed the jump button during which they can still jump if they
// hit the ground
const JUMP_INPUT_TIME = 0.1;
