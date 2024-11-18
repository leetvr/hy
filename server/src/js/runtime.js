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
  getEntityData: (entityId) => {
    return core.ops.get_entity_data(entityId);
  },
  spawnEntity: (entityTypeId, position, rotation, velocity) => {
    return core.ops.spawn_entity(entityTypeId, position, rotation, velocity);
  },
  despawnEntity: (entityId) => {
    return core.ops.despawn_entity(entityId);
  },
  checkMovementForCollisions: (playerID, currentPosition, movement) => {
    return core.ops.check_movement_for_collisions(playerID, currentPosition, movement);
  },
  anchorEntity: (entityId, anchorId, anchorName) => {
    return core.ops.anchor_entity(entityId, anchorId, anchorName);
  },
  detachEntity: (entityId) => {
    return core.ops.detach_entity(entityId);
  },
  interactEntity: (entityId, playerId, position, facingAngle) => {
    return core.ops.interact_entity(entityId, playerId, position, facingAngle);
  },
  getCollisionsForEntity: (entityId) => {
    return core.ops.get_collisions_for_entity(entityId);
  },
  getCollisionsForPlayer: (playerId) => {
    return core.ops.get_collisions_for_player(playerId);
  },
};
