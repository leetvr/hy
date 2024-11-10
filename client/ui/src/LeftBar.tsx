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
            <TestAudioManager engine={engine} />
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

// Make sure AudioManager is loading sound from Wasm
function TestAudioManager({ engine }: {engine: Engine}) {
    const [soundLoaded, setSoundLoaded] = useState(false);
  
    const loadAndPlaySound = () => {
      const soundUrl = "https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3";
      engine.load_and_play_sound(soundUrl)
        .then(() => {
          console.log('Sound loaded and is now playing');
          setSoundLoaded(true);
        })
        .catch(console.error);
    };
  
    return (
      <div>
        <button onClick={loadAndPlaySound}>Load and Play Wasm sound</button>
        {soundLoaded ? <div>Sound Loaded...</div> : <div>Sound Not Loaded</div>}
      </div>
    );
  }
  
