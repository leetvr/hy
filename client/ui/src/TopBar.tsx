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
          <img src="/client/ui/public/icon-play.svg" width="64" height="64" alt="⏵" />
        </span>
        <span
          id="pause-button"
          onClick={() => {
            alert("not implemented");
          }}
        >
          <img src="/client/ui/public/icon-pause.svg" width="64" height="64" alt="⏸" />
        </span>
        <span
          id="stop-button"
          onClick={() => {
            setMode(EngineMode.Edit);
          }}
        >
          <img src="/client/ui/public/icon-stop.svg" width="64" height="64" alt="⏹" />
        </span>
      </div>
    </div>
  );
}
