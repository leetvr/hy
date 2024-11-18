export const init = (worldState) => {
    let entities = hy.getEntities();
    let redSpawn = null;
    let blueSpawn = null;
    Object.keys(entities).forEach(entityId => {
        let entity_data = hy.getEntityData(entityId);
        const BLUE_FLAG_TYPE = 3;
        const RED_FLAG_TYPE = 4;
        if (blueSpawn == null && entity_data.entity_type == BLUE_FLAG_TYPE) {
            blueSpawn = entity_data.state.position;
            blueSpawn[1] += 1.;
        }
        if (redSpawn == null && entity_data.entity_type == RED_FLAG_TYPE) {
            redSpawn = entity_data.state.position;
            redSpawn[1] += 1.;
        }
    });
    worldState.blueSpawn = blueSpawn;
    worldState.redSpawn = redSpawn;
    worldState.redTeam = [];
    worldState.blueTeam = [];
    return worldState;
};
export const onAddPlayer = (worldState, playerId, playerState) => {
    // Give this man a gun
    let gun = hy.spawnEntity(1, [0, -0.5, -0.5], [0, 0, 0], [0, 0, 0]);
    hy.anchorEntity(gun, playerId, "hand_right_anchor");
    if (worldState.redTeam.length <= worldState.blueTeam.length) {
        playerState.customState.team = "red";
        worldState.redTeam.push(playerId);
        playerState.position = worldState.redSpawn;
    }
    else {
        playerState.customState.team = "blue";
        worldState.blueTeam.push(playerId);
        playerState.position = worldState.blueSpawn;
    }
    return [worldState, playerState];
};
