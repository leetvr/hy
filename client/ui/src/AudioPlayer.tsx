import { useState } from "react";
import { Engine } from "../../pkg/client";
import "./App.css";

export function TestStopSounds({engine}: {engine: Engine}) {
  const [stopError, setStopError] = useState<string>('');
  const [killError, setKillError] = useState<string>('');
  const [isStopSuccess, setIsStopSuccess] = useState<boolean>(false);
  const [isKillSuccess, setIsKillSuccess] = useState<boolean>(false);

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
  <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
    <div>
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
