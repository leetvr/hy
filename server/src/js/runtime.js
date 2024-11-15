// runtime.js
const { core } = Deno;

function argsToMessage(...args) {
  return args.map((arg) => JSON.stringify(arg)).join(" ");
}

globalThis.console = {
  log: (...args) => {
    core.print(`[out]: ${argsToMessage(...args)}\n`, false);
  },
  error: (...args) => {
    core.print(`[err]: ${argsToMessage(...args)}\n`, true);
  },
};

globalThis.hy = {
  getEntities: () => {
    return core.ops.get_entities();
  },
  isPlayerOnGround: (playerID) => {
    return core.ops.is_player_on_ground(playerID);
  },
  spawnEntity: (entityTypeId, position, velocity) => {
    return core.ops.spawn_entity(entityTypeId, position, velocity);
  },
  despawnEntity: (entityId) => {
    return core.ops.despawn_entity(entityId);
  },
  checkMovementForCollisions: (playerID, movement) => {
    return core.ops.check_movement_for_collisions(playerID, movement);
  },
  anchorEntity: (entityId, anchorId, anchorName, offset, rotation) => {
    return core.ops.anchor_entity(entityId, anchorId, anchorName, offset, rotation);
  },
  detachEntity: (entityId, position) => {
    return core.ops.detach_entity(entityId, position);
  },
  interactEntity: (entityId, playerId, position, facingAngle) => {
    return core.ops.interact_entity(entityId, playerId, position, facingAngle);
  }
};
