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
  spawnEntity: (entity_type_id, position) => {
    return core.ops.spawn_entity(entity_type_id, position);
  },
  despawnEntity: (entity_id) => {
    return core.ops.despawn_entity(entity_id);
  },
};
