// The "left bar": the block/entity palettes
import { useState } from "react";
import { EngineMode } from "../../pkg/client.js";

enum LeftBarTab {
    Blocks,
    Entities,
    Debug,
};

export default function LeftBar() {
    const [currentTab, setCurrentTab] = useState(LeftBarTab.Debug);
    let theContent = <p>hi - {currentTab}</p>;
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
