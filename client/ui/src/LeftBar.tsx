// The "left bar": the block/entity palettes
import { useEffect, useState } from "react";
import { EngineMode } from "../../pkg/client.js";
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
            {/* <AudioPlayer /> */}
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


// Check AudioManager is loading sound from wasm
function TestAudioManager({ engine }: { engine: Engine }) {
    const [soundLoaded, setSoundLoaded] = useState(false);
    useEffect(() => {
      console.log("Loading useEffect")
      if (engine) {
        const soundUrl = "https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3";
        engine.load_and_play_sound(soundUrl)
        .then(() => {
          console.log('Sound loaded and is now playing');
          setSoundLoaded(true); 
        })
        .catch(console.error);
      }
    }, [engine]); 
  
    if (!soundLoaded) {
      return <div>Sound Loading...</div>;
    } else {
      return <div>Sound Loaded...</div>; 
    }
  }