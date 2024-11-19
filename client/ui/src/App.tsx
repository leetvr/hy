import { useEffect, useState } from "react";
import init, { BlockRegistry, Engine, EngineMode, EntityTypeRegistry } from "../../pkg/client.js";
import "./App.css";
import LeftBar from "./LeftBar.tsx";
import RightBar from "./RightBar.tsx";
import CtfGameUi from "./CtfGameUi.tsx";
import TopBar from "./TopBar.tsx";

function App({ engine }: { engine: Engine }) {
  const initialEngineMode = EngineMode.Edit;
  const [currentMode, setModeState] = useState(initialEngineMode);
  const [blockRegistry, setBlockRegistry] = useState<BlockRegistry>();
  const [entityTypeRegistry, setEntityTypeRegistry] = useState<EntityTypeRegistry>();

  const setMode = (newMode: EngineMode) => {

    // WHenever the mode changes print some state data stuff
    console.log("Changing mode from", currentMode, "to", newMode);
    console.log("World state:\n", engine.ctx_get_world_state());
    console.log("Players: ", engine.ctx_get_players());

    if (newMode != currentMode) {
      setModeState(newMode);
      engine.ctx_set_engine_mode(newMode);
    }
  };

  useEffect(() => {
    engine.ctx_on_init((blockRegistry: BlockRegistry, entityTypeRegistry: EntityTypeRegistry) => {
      setBlockRegistry(blockRegistry);
      setEntityTypeRegistry(entityTypeRegistry);
    });

    setModeState(engine.ctx_get_engine_mode());
  }, [engine]);

  const editClass = getEngineModeText(currentMode);

  return (
    <div className={"mode-" + editClass}>
      <TopBar setMode={setMode} />
      {blockRegistry && entityTypeRegistry && (
        <LeftBar
          engine={engine}
          currentMode={currentMode}
          blockRegistry={blockRegistry}
          entityTypeRegistry={entityTypeRegistry}
        />
      )}
      <RightBar selectedEntity={false} />
      {currentMode === EngineMode.Play && <CtfGameUi
          redScore="4"
          blueScore="2"
          health="92"
          ammo="4"
          myTeam="red"
          iHaveFlag={false}
      />}
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
