// The "left bar": the block/entity palettes
import { useState } from "react";
import { Engine, EngineMode } from "../../pkg/client.js";
import BlockList from "./BlockList.tsx";
import Editor from "./Editor.js";

enum LeftBarTab {
    Blocks,
    Entities,
    Debug,
};

export default function LeftBar({ engine, currentMode, blockRegistry }: { Engine, EngineMode, BlockRegistry }) {
    const [currentTab, setCurrentTab] = useState(LeftBarTab.Debug);
    let theContent;
    if(currentTab === LeftBarTab.Blocks) {
        theContent = <BlockList blockRegistry={blockRegistry} setEngineBlockIndex={(idx) => { engine.ctx_set_editor_block_id(idx) }} />;
    } else if(currentTab === LeftBarTab.Entities) {
        theContent = <p>What even <i>is</i> an entity, man?</p>;
    } else {
        theContent = <div>
            {currentMode === EngineMode.Edit && <Editor engine={engine} blockRegistry={blockRegistry} />}
            {/* <TestAudioManager engine={engine} /> */}
            <AudioManagerWrapper engine={engine} />
        </div>;
    }
    // TODO: If we ever need to use it for anything else, this tab-bar business
    // can sensibly be separated into its own component
    return <div className="editor-panel editor-only" id="toolbox">
        <div className="tab-bar">
            <button
              className={currentTab == LeftBarTab.Blocks ? "tab-on" : ""}
              onClick={() => { setCurrentTab(LeftBarTab.Blocks); } }
            >Blocks</button>
            <button
              className={currentTab == LeftBarTab.Entities ? "tab-on" : ""}
              onClick={() => { setCurrentTab(LeftBarTab.Entities); } }
            >Entities</button>
            <button
              className={currentTab == LeftBarTab.Debug ? "tab-on" : ""}
              onClick={() => { setCurrentTab(LeftBarTab.Debug); } }
            >Debug</button>
        </div>
        {theContent}
    </div>;
}
function AudioManagerWrapper({ engine }: { engine: Engine }) {
  const [isLoaded, setIsLoaded] = useState(false);
  const [wasError, setWasError] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');

  const loadSound = async () => {
    try {
      const soundUrl = 'https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3';
      await engine.load_sound(soundUrl);
      console.log('Sound loaded');
      setIsLoaded(true);
      setWasError(false);
    } catch (error) {
      console.error('Error loading sound:', error);
      setWasError(true);
      setErrorMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div>
      {!isLoaded && (
        <div>
          <button onClick={loadSound}>Load Sound</button>
          {wasError && <div>Error loading sound: {errorMessage}</div>}
        </div>
      )}
      {isLoaded && <TestAudioManager engine={engine} />}
    </div>
  );
}

function TestAudioManager({ engine }: { engine: Engine }) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [wasError, setWasError] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');
  const [xPosition, setXPosition] = useState(0);

  const loadAndPlaySound = async () => {
    try {
      const soundUrl = 'https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3';
      await engine.load_sound(soundUrl);
      console.log('Sound loaded');
      engine.play_sound();
      console.log('Sound is now playing');
      setIsPlaying(true);
      setWasError(false);
    } catch (error) {
      console.error('Error loading or playing sound:', error);
      setWasError(true);
      setErrorMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const panLeft = () => {
    const newX = xPosition - 1;
    setXPosition(newX);
    engine.set_sound_position(newX, 0, 0);
  };

  const panRight = () => {
    const newX = xPosition + 1;
    setXPosition(newX);
    engine.set_sound_position(newX, 0, 0);
  };

  return (
    <div>
      <button onClick={loadAndPlaySound}>Play sound</button>
      {isPlaying ? <div>Sound Playing...</div> : <div>Sound Not Playing</div>}
      {wasError && <div>Error: {errorMessage}</div>}
      {isPlaying && (
        <div>
          <button onClick={panLeft}>Pan Left</button>
          <button onClick={panRight}>Pan Right</button>
          <p>Current X Position: {xPosition}</p>
        </div>
      )}
    </div>
  );
}
