import { Vec3, PlayerUpdate, PlayerControls, PlayerState, PlayerCollision, Vec2 } from "../lib/hy";

const GRAVITY = -20; // Gravity acceleration (m/s^2)
const MOVE_SPEED = 10.0; // Movement speed (units per second)
const JUMP_SPEED = 12.0; // Jump initial velocity (units per second)
const DT = 1 / 60; // Fixed delta time (seconds per frame)
const PLAYER_SIZE = [0.5, 1.5, 0.5]; // Player size (x, y, z)

export const update: PlayerUpdate = (
  currentState: PlayerState,
  controls: PlayerControls,
  collisions: PlayerCollision[],
): PlayerState => {
  const { position, velocity, animationState } = currentState;
  let newPosition: Vec3 = [...position];
  let newVelocity: Vec3 = [...velocity];
  let newAnimationState: string = animationState;

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
    // console.log(
    //   "Yaw",
    //   yaw,
    //   "sinYaw",
    //   sinYaw,
    //   "cosYaw",
    //   cosYaw,
    //   "normalizedInput",
    //   normalizedInput,
    //   "moveDirX",
    //   moveDirX,
    //   "moveDirZ",
    //   moveDirZ,
    // );

    // Update horizontal velocity
    newVelocity[0] = moveDirX * MOVE_SPEED;
    newVelocity[2] = -moveDirZ * MOVE_SPEED;

    newAnimationState = "run";
  } else {
    // Apply damping to horizontal velocity when no input
    newVelocity[0] *= 0.7; // Adjust damping factor as needed
    newVelocity[2] *= 0.7;
    newAnimationState = "idle";
  }

  // Handle jumping
  if (controls.jump && position[1] <= 1.01) {
    // Simple ground check
    newVelocity[1] = JUMP_SPEED;
    newAnimationState = "jump_pre";
  }

  // Apply gravity to vertical velocity
  newVelocity[1] += GRAVITY * DT;

  // Simple collision resolution
  collisions = collisions.filter((collision) => {
    return length(collision.resolution) < 0.5;
  });
  collisions.sort((a, b) => length(a.resolution) - length(b.resolution));
  if (collisions.length > 0) {
    const { block, resolution, normal } = collisions[0];
    // Move player out of block
    newPosition = [
      newPosition[0] + resolution[0],
      newPosition[1] + resolution[1],
      newPosition[2] + resolution[2],
    ];

    // Cancel velocity along the normal
    if (newVelocity[0] * normal[0] < 0) {
      newVelocity[0] = 0.;
    }
    if (newVelocity[1] * normal[1] < 0) {
      newVelocity[1] = 0.;
    }
    if (newVelocity[2] * normal[2] < 0) {
      newVelocity[2] = 0.;
    }

    if (normal[1] > 0. && newAnimationState == "jump" || newAnimationState == "jump_pre") {
      newAnimationState = "jump_post_light";
    }
  }

  // Update position based on velocity and delta time
  newPosition[0] += newVelocity[0] * DT;
  newPosition[1] += newVelocity[1] * DT;
  newPosition[2] += newVelocity[2] * DT;

  // Simple collision detection with the ground
  if (newPosition[1] < 1) {
    newPosition[1] = 1; // Make sure we're on the ground
    newVelocity[1] = 0; // Make sure we're no longer falling
  }

  return {
    position: newPosition,
    velocity: newVelocity,
    animationState: newAnimationState,
  };
};

function length(v: Vec3): number {
  return Math.hypot(v[0], v[1], v[2]);
}
