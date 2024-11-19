const GUN_TYPE_ID = 1;
const SHOTGUN_TYPE_ID = 5;
export const init = (worldState) => {
    let entities = hy.getEntities();
    let redSpawn = null;
    let blueSpawn = null;
    Object.keys(entities).forEach((entityId) => {
        let entity_data = entities[entityId];
        if (blueSpawn == null && entity_data.entity_type == BLUE_FLAG_TYPE) {
            blueSpawn = entity_data.state.position;
            blueSpawn[1] += 1;
        }
        if (redSpawn == null && entity_data.entity_type == RED_FLAG_TYPE) {
            redSpawn = entity_data.state.position;
            redSpawn[1] += 1;
        }
    });
    worldState.blueSpawn = blueSpawn;
    worldState.redSpawn = redSpawn;
    worldState.redTeam = [];
    worldState.blueTeam = [];
    worldState.redScore = 0;
    worldState.blueScore = 0;
    return worldState;
};
export const onAddPlayer = (worldState, playerId, playerState) => {
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
export const update = (worldState) => {
    // Flag capturing logic
    let entities = hy.getEntities();
    Object.keys(entities).forEach((leftId) => {
        Object.keys(entities).forEach((rightId) => {
            if (leftId == rightId) {
                return;
            }
            let l = entities[leftId];
            let r = entities[rightId];
            if ((l.entity_type == RED_FLAG_TYPE && r.entity_type == BLUE_FLAG_TYPE) ||
                (l.entity_type == BLUE_FLAG_TYPE && r.entity_type == RED_FLAG_TYPE)) {
                // One of the flags should be carried and the other not carried
                if (l.state.customState.carried == r.state.customState.carried) {
                    return;
                }
                if (distance(l.state.absolutePosition, r.state.absolutePosition) < 1) {
                    let carriedType;
                    if (l.state.customState.carried) {
                        carriedType = l.entity_type;
                    }
                    else {
                        carriedType = r.entity_type;
                    }
                    if (carriedType == RED_FLAG_TYPE) {
                        worldState.blueScore += 1;
                    }
                    else {
                        worldState.redScore += 1;
                    }
                    // Interacting with flags respawns them
                    hy.interactEntity(leftId, 0, [0, 0, 0], 0, 0);
                    hy.interactEntity(rightId, 0, [0, 0, 0], 0, 0);
                }
            }
        });
    });
    return worldState;
};
const distance = (l, r) => {
    return Math.hypot(l[0] - r[0], l[1] - r[1], l[2] - r[2]);
};
const BLUE_FLAG_TYPE = 3;
const RED_FLAG_TYPE = 4;
