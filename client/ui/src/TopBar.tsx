// The "top bar": play/pause/stop controls
import { EngineMode } from "../../pkg/client.js";

export default function TopBar({ setMode }: { setMode: (mode: EngineMode) => void }) {
  return (
    <div id="editorcontrols">
      <div className="ec-content editor-panel">
        <span
          id="play-button"
          onClick={() => {
            setMode(EngineMode.Play);
          }}
        >
          ⏵
        </span>
        <span
          id="pause-button"
          onClick={() => {
            alert("not implemented");
          }}
        >
          ⏸
        </span>
        <span
          id="stop-button"
          onClick={() => {
            setMode(EngineMode.Edit);
          }}
        >
          ⏹
        </span>
      </div>
    </div>
  );
}
