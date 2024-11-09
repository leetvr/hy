// The "left bar": the block/entity palettes
import { useState } from "react";
import { AudioPlayer } from "./AudioPlayer";
import BlockList from "./BlockList.tsx";
import { BlockRegistry, Engine, EngineMode } from "../../pkg/client.js";
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
        theContent = <BlockList blockRegistry={blockRegistry} />;
    } else if(currentTab === LeftBarTab.Entities) {
        theContent = <p>What even <i>is</i> an entity, man?</p>;
    } else {
        theContent = <div>
            <AudioPlayer />
            {currentMode === EngineMode.Edit && <Editor engine={engine} blockRegistry={blockRegistry} />}
        </div>;
    }
    // TODO: If we ever need to use it for anything else, this tab-bar business
    // can sensibly be separated into its own component
    return <div className="editor-panel" id="toolbox">
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
