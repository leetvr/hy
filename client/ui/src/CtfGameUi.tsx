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
    const [iHaveFlag, setIHaveFlag] = useState(false);
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
            if(iHaveFlag != freshPlayerInfo.get('hasFlag')) {
                setIHaveFlag(freshPlayerInfo.get('hasFlag'));
            }
        }, 150);
        return () => clearInterval(intervalId);
    });

    let otherTeam;
    if(playerTeam === "blue") {
        otherTeam = "red";
    } else {
        otherTeam = "blue";
    }
    let task;
    let taskClass = "";
    if(iHaveFlag) {
        taskClass = "has-flag";
        task = "You have the flag — return to base.";
    } else {
        task = "Infiltrate the " + otherTeam + " base and take their flag.";
    }

    let ammoLowClass = playerAmmo <= 3 ? "low" : "";
    let healthLowClass = playerHealth <= 1 ? "low" : "";
    return <div className={"ctf team-" + playerTeam}>
        <div className="status-ctr">
            <div className={"status status-health " + healthLowClass}><span>{playerHealth}</span></div>
            <div className={"status status-ammo " + ammoLowClass}><span>{playerAmmo}</span></div>
        </div>
        <div className="score-ctr">
            <div className="score score-blue">{blueScore}</div>
            <div className="score score-red">{redScore}</div>
        </div>
        <div className="instructions-ctr">
            <p className="instruction-whichteam">You are on <span class={"team-" + playerTeam}>{playerTeam}</span> team.</p>
            <p className={"instruction-task " + taskClass}>{task}</p>
        </div>
    </div>;
}
