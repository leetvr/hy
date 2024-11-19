import { createTypeReferenceDirectiveResolutionCache } from "typescript";
import { Vec3, PlayerUpdate, PlayerControls, PlayerState, Vec2, OnPlayerSpawn } from "../lib/hy";

const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 7.0; // Movement speed (units per second)
const JUMP_SPEED = 8.0; // Jump initial velocity (units per second)
const MIN_FALL_SPEED = -20.;
const DT = 1 / 60; // Fixed delta time (seconds per frame)

export const onSpawn: OnPlayerSpawn = (
  playerId: number,
  currentState: PlayerState,
): PlayerState => {
  const { customState, position } = currentState;
  let newCustomState = { ...customState };

  let newModelPath;
  if (customState.team == "red") {
    newModelPath = "kibble_ctf/player_red.gltf";
  } else {
    newModelPath = "kibble_ctf/player_blue.gltf";
  }

  newCustomState.maxHealth = MAX_HEALTH;
  newCustomState.health = MAX_HEALTH;
  newCustomState.spawnPosition = position;
  newCustomState.respawnTimer = RESPAWN_TIME;
  newCustomState.stunned = false;

  newCustomState.coyoteTime = 0.;
  newCustomState.jumpInputTime = 0.;

  newCustomState.itemPickupCooldowns = {};

  let gun = hy.spawnEntity(GUN_TYPE_ID, [0, 0, 0], [0, 0, 0], [0, 0, 0]);
  hy.anchorEntity(gun, playerId, "hand_right_anchor");
  newCustomState.ammo = max_ammo(GUN_TYPE_ID);
  newCustomState.maxAmmo = max_ammo(GUN_TYPE_ID);

  // NOTE(ll): modelPath *must* be set here, otherwise the model won't be loaded.
  return {
    ...currentState,
    customState: newCustomState,
    modelPath: newModelPath,
  };
};

export const update: PlayerUpdate = (
  playerID: number,
  currentState: PlayerState,
  controls: PlayerControls,
): PlayerState => {
  // Note(ll): I just put attachedEntities in currentState but mutating it in the script will not have any effect.
  // It's just a quick way to pass data to the script.
  const {
    position,
    velocity,
    animationState,
    facingAngle,
    isOnGround: wasOnGround,
    customState,
    attachedEntities,
  } = currentState;
  let newPosition: Vec3 = [...position];
  let newVelocity: Vec3 = [...velocity];
  let newFacingAngle: number = facingAngle;
  let newAnimationState: string = animationState;
  let newCustomState = { ...customState };
  let newControls = { ...controls };

  let isAlive = newCustomState.health > 0;
  if (!isAlive) {
    if (newCustomState.respawnTimer <= 0) {
      newPosition = newCustomState.spawnPosition;
      newCustomState.health = MAX_HEALTH;
      newCustomState.respawnTimer = 3.0;
    }
    newCustomState.respawnTimer -= DT;
  }

  const collisions = hy.getCollisionsForPlayer(playerID);
  let touchedEntities: { [key: string]: boolean } = {};
  let knockback = [0, 0, 0];
  collisions.forEach((collision) => {
    if (!isAlive) {
      return;
    }


    if (collision.collisionTarget == "entity") {
      let entityData = hy.getEntityData(collision.targetId);
      touchedEntities[collision.targetId] = true;
      if (newCustomState.itemPickupCooldowns[collision.targetId]) {
        return;
      }

      if (entityData != undefined) {
        if (entityData.entity_type == GUN_TYPE_ID || entityData.entity_type == SHOTGUN_TYPE_ID) {
          // Pick up gun if it's different from the one in the hand
          let heldItemId = null;
          let heldItemType: number | null = null;
          if (attachedEntities["hand_right_anchor"]) {
            heldItemId = attachedEntities["hand_right_anchor"][0];
            let heldItemData = hy.getEntityData(heldItemId);
            heldItemType = heldItemData.entity_type;
          }
          let itemData = hy.getEntityData(collision.targetId);
          if (itemData.state.anchor != null) {
            return;
          }
          if (itemData.entity_type == heldItemType) {
            return;
          }


          hy.anchorEntity(collision.targetId, playerID, "hand_right_anchor");
          newCustomState.ammo = max_ammo(itemData.entity_type);
          newCustomState.maxAmmo = max_ammo(itemData.entity_type);

          if (heldItemId) {
            // Drop the gun that was previously held, in the same position as the picked up gun
            let dropPosition: Vec3 = [
              itemData.state.customState.spawnPosition[0],
              itemData.state.customState.spawnPosition[1],
              itemData.state.customState.spawnPosition[2]
            ];
            dropPosition[1] -= 0.75;
            hy.detachEntity(heldItemId, dropPosition);
            newCustomState.itemPickupCooldowns[heldItemId] = 5;
            touchedEntities[heldItemId] = true;
          }
        }

        if (entityData.entity_type == AMMO_TYPE_ID) {
          if (newCustomState.ammo < newCustomState.maxAmmo) {
            hy.despawnEntity(collision.targetId);
            newCustomState.ammo = newCustomState.maxAmmo;
          }
        }

        if (entityData.entity_type == BANDAGE_TYPE_ID) {
          if (newCustomState.health < newCustomState.maxHealth) {
            hy.despawnEntity(collision.targetId);
            newCustomState.health = newCustomState.maxHealth;
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

          // Get knocked away from the ball
          const ballVelocity = [entityData.state.velocity[0], entityData.state.velocity[2]];
          const length = Math.hypot(ballVelocity[0], ballVelocity[1]);
          if (length > 0) {
            let normalizedBallVelocity = [
              ballVelocity[0] / length,
              ballVelocity[1] / length,
            ];
            knockback = [
              normalizedBallVelocity[0] * 10,
              5,
              normalizedBallVelocity[1] * 10
            ];
            newCustomState.stunned = true;
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
          } else {
            flag_team = "red";
          }

          if (newCustomState.team == flag_team) {
            // Interacting with a flag returns it to its spawn
            hy.interactEntity(collision.targetId, playerID, position, newControls.camera_yaw, newControls.camera_pitch);
          } else {
            // Pick up the flag if we aren't already holding something in the left hand
            if (!newCustomState.stunned && !attachedEntities["back_anchor"]) {
              hy.anchorEntity(collision.targetId, playerID, "back_anchor");
            }
          }
        }
      }
    }
  });

  if (!isAlive || newCustomState.stunned) {
    if (attachedEntities["back_anchor"]) {
      hy.detachEntity(attachedEntities["back_anchor"][0], newPosition);
    }

    // Reset controls when player is dead or stunned
    newControls.move_direction = [0, 0];
    newControls.jump = false;
    newControls.fire = false;
  }

  // Items that are no longer in contact with the player should be removed from the dontPickupItem
  // list
  Object.keys(newCustomState.itemPickupCooldowns).forEach((itemId) => {
    if (!touchedEntities[itemId]) {
      newCustomState.itemPickupCooldowns[itemId] -= 1;
      if (newCustomState.itemPickupCooldowns[itemId] <= 0) {
        delete newCustomState.itemPickupCooldowns[itemId];
      }
    } else {
      newCustomState.itemPickupCooldowns[itemId] = 5;
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
    const normalizedInput: Vec2 = [inputX / inputLength, inputZ / inputLength];

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

  } else {
    // TODO: Apply damping to horizontal velocity when no input
    if (wasOnGround && !newCustomState.stunned) {
      newVelocity[0] *= 0.7;
      newVelocity[2] *= 0.7;
    }
  }

  // Apply gravity
  newVelocity[1] += GRAVITY * DT;

  if (knockback[0] != 0 || knockback[1] != 0 || knockback[2] != 0) {
    console.log("knockback", knockback);
    newVelocity[0] = knockback[0];
    newVelocity[1] = knockback[1];
    newVelocity[2] = knockback[2];
  }

  // Update position based on velocity and delta time
  const desiredMovement: Vec3 = [newVelocity[0], newVelocity[1], newVelocity[2]];

  const { correctedMovement, isOnGround } = hy.checkMovementForCollisions(
    playerID,
    position,
    desiredMovement,
  );

  newVelocity[0] = correctedMovement[0];
  newVelocity[1] = correctedMovement[1];
  newVelocity[2] = correctedMovement[2];

  newVelocity[1] = Math.max(newVelocity[1], MIN_FALL_SPEED);

  if (isOnGround && newVelocity[1] < 0) {
    newCustomState.stunned = false;
    newCustomState.coyoteTime = COYOTE_TIME;
  } else {
    newCustomState.coyoteTime = Math.max(0., newCustomState.coyoteTime - DT);
  }
  if (controls.jump) {
    newCustomState.jumpInputTime = JUMP_INPUT_TIME;
  } else {
    newCustomState.jumpInputTime = Math.max(0., newCustomState.jumpInputTime - DT);
  }

  let didJump = false;
  if (newCustomState.coyoteTime > 0. && newCustomState.jumpInputTime > 0.) {
    didJump = true;
    newVelocity[1] = JUMP_SPEED;
    newCustomState.coyoteTime = 0.;
  }

  // Special jump pad logic
  if (isOnGround && hy.getBlock([position[0], position[1] - 1.0, position[2]]) == 11) {
    newVelocity[1] = JUMP_SPEED * 2;
  }

  newPosition[0] += newVelocity[0] * DT;
  newPosition[1] += newVelocity[1] * DT;
  newPosition[2] += newVelocity[2] * DT;

  if (!isAlive) {
    newAnimationState = "sleep";
  } else if (isFiring) {
    newAnimationState = "simple_interact";
  } else if (didJump) {
    newAnimationState = "jump_loop";
  } else if ((inputX !== 0 || inputZ !== 0) && isOnGround) {
    newAnimationState = "run";
  } else if (isOnGround) {
    newAnimationState = "idle";
  }

  // Players die when they are below the map
  if (newPosition[1] < -10) {
    newCustomState.health = 0;
  }

  return {
    ...currentState,
    position: newPosition,
    velocity: newVelocity,
    facingAngle: newFacingAngle,
    animationState: newAnimationState,
    customState: newCustomState,
    isOnGround,
    attachedEntities,
  };
};

const GUN_TYPE_ID = 1;
const BALL_TYPE_ID = 2;
const BLUE_FLAG_TYPE_ID = 3;
const RED_FLAG_TYPE_ID = 4;
const SHOTGUN_TYPE_ID = 5;
const BULLET_TYPE_ID = 6;
const AMMO_TYPE_ID = 7;
const BANDAGE_TYPE_ID = 8;

function max_ammo(entity_type: number): number {
  if (entity_type == GUN_TYPE_ID) {
    return 10;
  } else if (entity_type == SHOTGUN_TYPE_ID) {
    return 3;
  } else {
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