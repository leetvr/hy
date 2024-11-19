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
  getPlayerState: core.ops.get_player_state,
  getEntities: () => {
    return core.ops.get_entities();
  },
  getEntityData: (entityId) => {
    return core.ops.get_entity_data(entityId);
  },
  spawnEntity: core.ops.spawn_entity,
  despawnEntity: (entityId) => {
    return core.ops.despawn_entity(entityId);
  },
  checkMovementForCollisions: (playerID, currentPosition, movement) => {
    return core.ops.check_movement_for_collisions(playerID, currentPosition, movement);
  },
  anchorEntity: (entityId, anchorId, anchorName) => {
    return core.ops.anchor_entity(entityId, anchorId, anchorName);
  },
  detachEntity: (entityId, position) => {
    return core.ops.detach_entity(entityId, position);
  },
  interactEntity: core.ops.interact_entity,
  getCollisionsForEntity: (entityId) => {
    return core.ops.get_collisions_for_entity(entityId);
  },
  getCollisionsForPlayer: (playerId) => {
    return core.ops.get_collisions_for_player(playerId);
  },
  playSound: (soundId, position, volume) => {
    return core.ops.play_sound(soundId, position, volume);
  },
};
