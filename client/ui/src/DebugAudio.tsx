import { useState } from "react";
import { Engine } from "../../pkg/client";
import "./App.css";

export function DebugAudio({
  engine,
}: {engine: Engine}) {
  // Use `fixed` positioning to remove from document flow
  // to avoid impacting the rest of the layout
  const audioWrapperStyle: React.CSSProperties = {
    position: 'fixed',
    top: '10%',
    right: '5%',
    width: '200px',
    backgroundColor: '#d9d9d9',
    border: '1px solid #ccc',
    padding: '20px',
    borderRadius: '8px',
    boxShadow: '0 4px 6px rgba(0,0,0,0.1)',
    zIndex: 1000
  };

  return (
    <div style={{
      ...audioWrapperStyle,
      display: engine.is_audio_manager_debug() ? 'block' : 'none'
    }}>
      {engine.is_audio_manager_debug() && <DebugAudioBar engine={engine} />}
    </div>
  );
}

function DebugAudioBar({engine}: {engine: Engine}) {
  const [stopError, setStopError] = useState<string>('');
  const [killError, setKillError] = useState<string>('');
  const [isStopSuccess, setIsStopSuccess] = useState<boolean>(false);
  const [isKillSuccess, setIsKillSuccess] = useState<boolean>(false);

  const handleLoadSounds = async () => {
    try {
      engine.load_sound("pain");
      console.log('Sound loaded');
    } catch (error) {
      console.error("Error on loud_sound: {}", error); 
    }
  }

  const handleStopAllSounds = () => {
    try {
      engine.stop_sounds();
      console.log("All sounds stopped.");
      setIsStopSuccess(true);
      setStopError('');
    } catch (error) {
      console.error("Error stopping all sounds:", error);
      setStopError(error instanceof Error ? error.message : String(error));
      setIsStopSuccess(false);
      
    }
  };
  
  const handleClearSoundsBank = () => {
    try {
      engine.kill_sounds();
      console.log("Sounds bank cleared.");
      setIsKillSuccess(true);
      setKillError('');
    } catch (error) {
      console.error("Error clearing sounds bank:", error);
      setKillError(error instanceof Error ? error.message : String(error));
      setIsKillSuccess(false);
    }
  };

  return (
  <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
    <h3>Audio Debugging</h3>
    <div>
      {engine.is_audio_manager_debug() && (
        <div style={{ color: 'black', marginTop: '8px' }}>
        Left click on block to spawn a sound or right click to spawn sound @KaneFace
        </div>
        )}
    </div>
    <div>
      <button onClick={handleLoadSounds}>Load {"pain"} sound</button>
    </div>
    <div>
        <div style={{ color: 'black', marginTop: '8px' }}>
          Pan all sounds along the X axis
        </div>
        <button onClick={() => engine.move_all_panner_nodes(-5.0)}>Pan Left</button>
        <button onClick={() => engine.move_all_panner_nodes(5.0)}>Pan Right</button>
    </div>
    <div>
      <button onClick={handleStopAllSounds}>
            Stop All Sounds
          </button>
          
          {stopError && (
            <div style={{ color: 'red', marginTop: '8px' }}>
              Error: {stopError}
            </div>
          )}
          
          {isStopSuccess && (
            <div style={{ color: 'green', marginTop: '8px' }}>
              All sounds have been stopped successfully.
            </div>
          )}
    </div>

    <div>
        <button onClick={handleClearSoundsBank}>
          Clear Sounds Bank
        </button>
        
        {killError && (
          <div style={{ color: 'red', marginTop: '8px' }}>
            Error: {killError}
          </div>
        )}
        
        {isKillSuccess && (
          <div style={{ color: 'green', marginTop: '8px' }}>
            Sounds bank has been cleared successfully.
          </div>
        )}
      </div>
  </div>
  );
}
