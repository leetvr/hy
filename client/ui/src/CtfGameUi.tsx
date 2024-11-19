import { useEffect, useState } from "react";
import { Engine } from "../../pkg/client.js";

export default function CtfGameUi({
    engine
}: {
    engine: Engine
}) {
    const [blueScore, setBlueScore] = useState(0);
    const [redScore, setRedScore] = useState(0);
    const [playerHealth, setPlayerHealth] = useState(0);
    const [playerAmmo, setPlayerAmmo] = useState(0);
    const [playerTeam, setPlayerTeam] = useState("red");
    useEffect(() => {
        const intervalId = setInterval( () => {
            let freshWorldState = engine.ctx_get_world_state();
            if(blueScore != freshWorldState.get('blueScore')) {
                setBlueScore(freshWorldState.get('blueScore'));
            }
            if(redScore != freshWorldState.get('redScore')) {
                setRedScore(freshWorldState.get('redScore'));
            }

            const playerId = engine.ctx_get_my_player_id();
            let freshPlayerInfo = engine.ctx_get_players().get(playerId);
            if(playerHealth != freshPlayerInfo.get('health')) {
                setPlayerHealth(freshPlayerInfo.get('health'));
            }
            if(playerAmmo != freshPlayerInfo.get('ammo')) {
                setPlayerAmmo(freshPlayerInfo.get('ammo'));
            }
            if(playerTeam != freshPlayerInfo.get('team')) {
                setPlayerTeam(freshPlayerInfo.get('team'));
            }
        }, 150);
        return () => clearInterval(intervalId);
    });

    let task;
    let otherTeam;
    if(playerTeam === "blue") {
        otherTeam = "red";
    } else {
        otherTeam = "blue";
    }
    const iHaveFlag = false;
    if(iHaveFlag) {
        task = "You have the flag — return to base.";
    } else {
        task = "Infiltrate the " + otherTeam + " base and take their flag.";
    }
    return <div className="ctf">
        <div className="status-ctr">
            <div className="status status-health">{playerHealth}</div>
            <div className="status status-ammo">{playerAmmo}</div>
        </div>
        <div className="score-ctr">
            <div className="score score-blue">{blueScore}</div>
            <div className="score score-red">{redScore}</div>
        </div>
        <div className="instructions-ctr">
            <p className="instruction-whichteam">You are on <span class={"team-" + playerTeam}>{playerTeam}</span> team.</p>
            <p className="instruction-task">{task}</p>
        </div>
    </div>;
}
