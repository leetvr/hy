import { Vec3, PlayerUpdate, PlayerControls, PlayerState, PlayerCollision, Vec2 } from "../lib/hy";

const GRAVITY = -9.81; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 5.0; // Movement speed (units per second)
const JUMP_SPEED = 5.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)

export const update: PlayerUpdate = (
  playerID: number,
  currentState: PlayerState,
  controls: PlayerControls,
  collisions: PlayerCollision[],
): PlayerState => {
  const { position, velocity, animationState, isOnGround: wasOnGround } = currentState;
  let newPosition: Vec3 = [...position];
  let newVelocity: Vec3 = [...velocity];
  let newAnimationState: string = animationState;

  if (controls.fire) {
    let gun = hy.spawnEntity(1, [0, -0.5, -0.5], [0, 0, 0], [0, 0, 0]);
    hy.anchorEntity(gun, playerID, "hand_right_anchor");
    hy.interactEntity(gun, playerID, position, controls.camera_yaw);
  }

  // Handle horizontal movement
  const inputX = controls.move_direction[0];
  const inputZ = controls.move_direction[1];

  if (inputX !== 0 || inputZ !== 0) {
    // Normalize input direction
    const inputLength = Math.hypot(inputX, inputZ);
    const normalizedInput: Vec2 = [inputX / inputLength, inputZ / inputLength];

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
  } else {
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
  const desiredMovement: Vec3 = [newVelocity[0], newVelocity[1], newVelocity[2]];

  const { correctedMovement, isOnGround } = hy.checkMovementForCollisions(
    playerID,
    position,
    desiredMovement,
  );

  newVelocity[0] = correctedMovement[0];
  newVelocity[1] = correctedMovement[1];
  newVelocity[2] = correctedMovement[2];

  if (isOnGround && controls.jump) {
    newVelocity[1] = JUMP_SPEED;
  }

  newPosition[0] += newVelocity[0] * DT;
  newPosition[1] += newVelocity[1] * DT;
  newPosition[2] += newVelocity[2] * DT;

  return {
    position: newPosition,
    velocity: newVelocity,
    animationState: newAnimationState,
    isOnGround,
  };
};
