// The "left bar": the block/entity palettes
import { useState } from "react";
import { BlockRegistry, Engine, EngineMode, EntityTypeRegistry } from "../../pkg/client.js";
import { TestStopSounds } from "./AudioPlayer.tsx";
import BlockList from "./BlockList.tsx";
import Editor from "./Editor.js";
import EntityTypeList from "./EntityTypeList.tsx";

enum LeftBarTab {
    Blocks,
    Entities,
    Debug,
};

export default function LeftBar({ engine, currentMode, blockRegistry, entityTypeRegistry }: { engine: Engine, currentMode: EngineMode, blockRegistry: BlockRegistry, entityTypeRegistry: EntityTypeRegistry }) {
    const [currentTab, setCurrentTab] = useState(LeftBarTab.Debug);
    let theContent;
    if (currentTab === LeftBarTab.Blocks) {
        theContent = <BlockList blockRegistry={blockRegistry} setEngineBlockIndex={(idx) => { engine.ctx_set_editor_block_id(idx) }} />;
    } else if (currentTab === LeftBarTab.Entities) {
        theContent = <EntityTypeList entityTypeRegistry={entityTypeRegistry} setEngineEntityIndex={(idx) => { engine.ctx_set_editor_entity_type_id(idx) }} />;
    } else {
        theContent = <div>
            {currentMode === EngineMode.Edit && <Editor engine={engine} blockRegistry={blockRegistry} />}
            <TestStopSounds engine={engine} />
            </div>
    }
    // TODO: If we ever need to use it for anything else, this tab-bar business
    // can sensibly be separated into its own component
    return <div className="editor-panel editor-only" id="toolbox">
        <div className="tab-bar">
            <button
                className={currentTab == LeftBarTab.Blocks ? "tab-on" : ""}
                onClick={() => { setCurrentTab(LeftBarTab.Blocks); }}
            >Blocks</button>
            <button
                className={currentTab == LeftBarTab.Entities ? "tab-on" : ""}
                onClick={() => { setCurrentTab(LeftBarTab.Entities); }}
            >Entities</button>
            <button
                className={currentTab == LeftBarTab.Debug ? "tab-on" : ""}
                onClick={() => { setCurrentTab(LeftBarTab.Debug); }}
            >Debug</button>
        </div>
        {theContent}
    </div>;
}
