export default function CtfGameUi({
    blueScore,
    redScore,
    health,
    ammo,
    myTeam,
    iHaveFlag,
}: {
    blueScore: number,
    redScore: number,
    health: number,
    ammo: number,
    myTeam: string,
    iHaveFlag: boolean,
}) {
    let task;
    let otherTeam;
    if(myTeam === "blue") {
        otherTeam = "red";
    } else {
        otherTeam = "blue";
    }
    if(iHaveFlag) {
        task = "You have the flag — return to base.";
    } else {
        task = "Infiltrate the " + otherTeam + " base and take their flag.";
    }
    return <div className="ctf">
        <div className="status-ctr">
            <div className="status status-health">{health}</div>
            <div className="status status-ammo">{ammo}</div>
        </div>
        <div className="score-ctr">
            <div className="score score-blue">{blueScore}</div>
            <div className="score score-red">{redScore}</div>
        </div>
        <div className="instructions-ctr">
            <p className="instruction-whichteam">You are on <span class={"team-" + myTeam}>{myTeam}</span> team.</p>
            <p className="instruction-task">{task}</p>
        </div>
    </div>;
}
