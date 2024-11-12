import { useEffect, useState } from "react";
import init, { BlockRegistry, Engine, EngineMode, EntityTypeRegistry } from "../../pkg/client.js";
import "./App.css";
import LeftBar from "./LeftBar.tsx";
import RightBar from "./RightBar.tsx";
import TopBar from "./TopBar.tsx";

function App({ engine }: { engine: Engine }) {
  const initialEngineMode = EngineMode.Edit;
  const [currentMode, setModeState] = useState(initialEngineMode);
  const [blockRegistry, setBlockRegistry] = useState<BlockRegistry>();
  const [entityTypeRegistry, setEntityTypeRegistry] = useState<EntityTypeRegistry>();

  useEffect(() => {
    engine.ctx_on_init((blockRegistry: BlockRegistry, entityTypeRegistry: EntityTypeRegistry) => {
      setBlockRegistry(blockRegistry);
      setEntityTypeRegistry(entityTypeRegistry);
    });
  }, [engine]);

  const setMode = (newMode: EngineMode) => {
    if (newMode != currentMode) {
      setModeState(newMode);
      engine.ctx_set_engine_mode(newMode);
    }
  };

  const editClass = getEngineModeText(currentMode);

  return (
    <div className={"mode-" + editClass}>
      <TopBar setMode={setMode} />
      {blockRegistry && entityTypeRegistry && (
        <LeftBar engine={engine} currentMode={currentMode} blockRegistry={blockRegistry} entityTypeRegistry={entityTypeRegistry} />
      )}
      <RightBar selectedEntity={false} />
    </div>
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
        
        // Should we be loading sounds elsewhere?
        // await engine.load_sounds_into_bank();

        const tick = (timestamp: number) => {
          engine.tick(timestamp);
          window.requestAnimationFrame(tick);
        };

        window.requestAnimationFrame(tick);

        // Mouse lock and input
        const on_mouse_move = (event: MouseEvent) => {
          engine.mouse_move(event);
        };
        const on_key_down = (event: KeyboardEvent) => {
          engine.key_down(event);
          event.preventDefault();
        };
        const on_key_up = (event: KeyboardEvent) => {
          engine.key_up(event);
          event.preventDefault();
        };
        const on_mouse_down = (event: MouseEvent) => {
          engine.mouse_down(event);
          event.preventDefault();
        };
        const on_mouse_up = (event: MouseEvent) => {
          engine.mouse_up(event);
          event.preventDefault();
        };

        const canvas = engine.ctx_get_canvas();
        canvas.addEventListener("click", async (event) => {
          event.preventDefault();
          await canvas.requestPointerLock({
            unadjustedMovement: true,
          });
        });

        new ResizeObserver((entries) => {
          for (const entry of entries) {
            if (entry.target !== canvas) {
              console.warn("Unexpected resize observer target", entry.target);
              continue;
            }

            const size = entry.devicePixelContentBoxSize[0];

            const height = size.blockSize;
            const width = size.inlineSize;

            canvas.width = width;
            canvas.height = height;

            engine.resize(width, height);
          }
        }).observe(canvas);

        document.addEventListener("pointerlockchange", () => {
          if (document.pointerLockElement === canvas) {
            window.addEventListener("keydown", on_key_down);
            window.addEventListener("keyup", on_key_up);
            canvas.addEventListener("mousemove", on_mouse_move);
            canvas.addEventListener("mousedown", on_mouse_down);
            canvas.addEventListener("mouseup", on_mouse_up);
          } else {
            window.removeEventListener("keydown", on_key_down);
            window.removeEventListener("keyup", on_key_up);
            canvas.removeEventListener("mousemove", on_mouse_move);
            canvas.removeEventListener("mousedown", on_mouse_down);
            canvas.removeEventListener("mouseup", on_mouse_up);
          }
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
