// The "top bar": play/pause/stop controls
import EngineMode from "../../pkg/client.js";

export default function TopBar({ editorMode }: { editorMode: EngineMode }) {
    return <div id="editorcontrols">
        <div className="ec-content editor-panel">
            <span id="play-button">
             ⏵
            </span>
            <span id="pause-button">
             ⏸
            </span>
            <span id="stop-button">
             ⏹
            </span>
        </div>
    </div>;
}
