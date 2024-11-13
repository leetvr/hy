import { useState } from "react";
import { Engine } from "../../pkg/client";
import "./App.css";

export function DebugAudioManager({engine}: {engine: Engine}) {
  const [stopError, setStopError] = useState<string>('');
  const [killError, setKillError] = useState<string>('');
  const [isStopSuccess, setIsStopSuccess] = useState<boolean>(false);
  const [isKillSuccess, setIsKillSuccess] = useState<boolean>(false);

  const [loadError, setLoadError] = useState(false);
  // const sound_name = ;

  const handleLoadSounds = () => {
    try {
      engine.load_sound("pain");
      console.log('Sound loaded');
      setLoadError(false);
    } catch (error) {
      console.error("Error on loud_sound: {}", error); 
      setLoadError(true);
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
        Left click on block to spawn a sound
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
        <button onClick={() => engine.update_sound_positions(-5.0)}>Pan Left</button>
        <button onClick={() => engine.update_sound_positions(5.0)}>Pan Right</button>
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
