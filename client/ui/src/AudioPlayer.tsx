import { useEffect, useState } from "react";
import "./App.css";

// Whatever, it was in the tutorial
// TODO: make it work with local sound asset
const sampleSound = "https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3";

// Just using a simple component for now to see how it all fits together
export function AudioPlayer() {
  // AudioPlayer state
  const [audioContext, setAudioContext] = useState<AudioContext | null>(null);
  const [buffer, setBuffer] = useState<AudioBuffer | null>(null);
  const [source, setSource] = useState<AudioBufferSourceNode | null>(null);
  const [panner, setPanner] = useState<PannerNode | null>(null);

  // Playback state
  const [isPlaying, setIsPlaying] = useState(false); 
  // Panning state
  const [isMovingLeft, setIsMovingLeft] = useState(false);
  const [isMovingRight, setIsMovingRight] = useState(false);
  const [xPosition, setXPosition] = useState(0); 
  // Effects state
  const [isDistortionEnabled, setIsDistortionEnabled] = useState(false);
  const [distortion, setDistortion] = useState<WaveShaperNode | null>(null);

  // Initialize context, buffer and load sound
  useEffect(() => {
    const context = new (window.AudioContext)();
    setAudioContext(context);

    const loadAudio = async () => {
      const response = await fetch(sampleSound);
      const arrayBuffer = await response.arrayBuffer();
      const decodedBuffer = await context.decodeAudioData(arrayBuffer);
      setBuffer(decodedBuffer);
    };
    loadAudio();

    return () => {
      if (audioContext) {
        audioContext.close(); // Cleanup on unmount
      }
    };
		// Empty dependency array as only want to initialise when mounting
  }, []); 

  // Play or stop sound at the specified x, y, z position
  const toggleSound = (x: number, y: number, z: number) => {
    if (!audioContext || !buffer) {
      console.log("Audio context or buffer is not initialized");
      return;
    }

    if (isPlaying) {
      // Stop the sound
      source?.stop();
      setSource(null);
      setPanner(null);
      setIsPlaying(false);
			// Reset sound parameters... (these should probably be passed down to a child `Sound` component or something to persist the state)
      setIsMovingLeft(false);
      setIsMovingRight(false);
      setXPosition(0); 
      setIsDistortionEnabled(false); 
    } else {
      // Play the sound
      const newSource = audioContext.createBufferSource();
      newSource.buffer = buffer;

      const newPanner = audioContext.createPanner();
      newPanner.positionX.value = x;
      newPanner.positionY.value = y;
      newPanner.positionZ.value = z;

      newSource.connect(newPanner);
      newPanner.connect(audioContext.destination);

      newSource.start();

      // Keep references to source and panner
      setSource(newSource);
      setPanner(newPanner);
      setIsPlaying(true);
    }
  };

  const applyDistortion = () => {
    if (!audioContext || !source || !panner) return;

    // Create a new distortion node only if it's not already applied
    if (!distortion) {
      const newDistortion = audioContext.createWaveShaper();
			// Do the distortion logic
      const curve = new Float32Array(44100);
      for (let i = 0; i < curve.length; i++) {
        const x = i * 2 / curve.length - 1;  
        curve[i] = Math.tanh(x * 10);  
      }
      newDistortion.curve = curve;
			// This can be reduced for less cost
      newDistortion.oversample = '4x';

      // Connect the distortion between the source and panner
      source.connect(newDistortion);
      newDistortion.connect(panner);
      panner.connect(audioContext.destination);

			// Save the distortion node so we can remove it later
      setDistortion(newDistortion);  
    }
  };

  const removeDistortion = () => {
    if (!audioContext) return;

    if (distortion && source && panner) {
      source.disconnect(distortion);
      distortion.disconnect(panner);

      source.connect(panner);
      panner.connect(audioContext.destination);

      setDistortion(null); 
    }
  };

  const toggleDistortion = () => {
    if (isDistortionEnabled) {
      removeDistortion();
    } else {
      applyDistortion();
    }
    setIsDistortionEnabled(prevState => !prevState);
  }


  // Handlers to move sound position left and right when button is held
  const handleMoveLeftDown = () => {
    setIsMovingLeft(true);
  };

  const handleMoveLeftUp = () => {
    setIsMovingLeft(false);
  };

  const handleMoveRightDown = () => {
    setIsMovingRight(true);
  };

  const handleMoveRightUp = () => {
    setIsMovingRight(false);
  };

	const pan_speed = 50;
  // Update sound position every `pan_speed` (ms) when button is held
  useEffect(() => {
    const moveInterval = setInterval(() => {
      if (isMovingLeft) {
        const newX = xPosition - 1;
        setXPosition(newX);
        if (panner) {
          panner.positionX.value = newX;
        }
      } else if (isMovingRight) {
        const newX = xPosition + 1;
        setXPosition(newX);
        if (panner) {
          panner.positionX.value = newX;
        }
      }
    }, pan_speed);

    return () => clearInterval(moveInterval); // Cleanup on unmount
  }, [isMovingLeft, isMovingRight, xPosition, panner]);

  return (
    <div>
      <button onClick={() => toggleSound(0, 0, 0)}>
        {isPlaying ? "Stop Sound" : "Play Sound"}
      </button>

      {isPlaying && (
        <div>
          <button
            onMouseDown={handleMoveLeftDown}
            onMouseUp={handleMoveLeftUp}
            onMouseLeave={handleMoveLeftUp} 
          >
            Move Left
          </button>
          <button
            onMouseDown={handleMoveRightDown}
            onMouseUp={handleMoveRightUp}
            onMouseLeave={handleMoveRightUp} 
          >
            Move Right
          </button>
          
          <button onClick={toggleDistortion}>
            {isDistortionEnabled ? "Disable Distortion" : "Enable Distortion"}
          </button>

          <p>Current X Position: {xPosition}</p>
        </div>
      )}

      
    </div>
  );
}