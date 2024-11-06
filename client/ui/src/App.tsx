import { useEffect, useState } from "react";
import "./App.css";
import { AudioPlayer } from "./AudioPlayer";
import init, { Engine } from "../../pkg/client.js";

// sigh: https://github.com/rustwasm/wasm-bindgen/issues/2407
enum EngineMode {
  Play = 0,
  Edit = 1,
}

function App({ engine }: { engine: Engine }) {
  const initialEngineMode = EngineMode.Edit;
  const [currentMode, setMode] = useState(initialEngineMode);

  useEffect(() => {
    console.log("App has been mounted probably");
  }, []);

  const handleClick = () => {
    const nextMode = nextEngineMode(currentMode);
    setMode(nextMode);
    engine.ctx_set_engine_mode(nextMode);
  };

  return (
    <>
      <div className="card">
        <p>
          We are currently in <strong>{getEngineModeText(currentMode)}</strong> mode
        </p>
        <button onClick={handleClick}>
          Switch to {getEngineModeText(nextEngineMode(currentMode))}
        </button>
        <AudioPlayer />
      </div>
    </>
  );
}

const getEngineModeText = (mode: EngineMode): string => {
  switch (mode) {
    case EngineMode.Play:
      return "Play";
    case EngineMode.Edit:
      return "Edit";
  }
};

const nextEngineMode = (mode: EngineMode): EngineMode => {
  switch (mode) {
    case EngineMode.Play:
      return EngineMode.Edit;
    case EngineMode.Edit:
      return EngineMode.Play;
  }
};

function WasmWrapper() {
  const [engine, setEngine] = useState<Engine>();

  useEffect(() => {
    async function load() {
      try {
        await init(); // init
        const engine = Engine.new();

        const tick = (timestamp: number) => {
          engine.tick(timestamp);
          window.requestAnimationFrame(tick);
        };

        window.requestAnimationFrame(tick);
        window.addEventListener("keydown", (event) => {
          engine.key_down(event);
        });

        window.addEventListener("keyup", (event) => {
          engine.key_up(event);
        });

        setEngine(engine);
        console.log("Engine loaded successfully!");
      } catch (err) {
        console.error("Unable to load module!", err);
      }
    }

    load();
  }, []);

  if (!engine) {
    return <div>Loading..</div>;
  }

  return <App engine={engine} />;
}

export default WasmWrapper;
