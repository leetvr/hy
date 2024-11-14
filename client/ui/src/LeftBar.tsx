// The "left bar": the block/entity palettes
import { useState } from "react";
import BlockList from "./BlockList.tsx";
import EntityTypeList from "./EntityTypeList.tsx";
import { BlockRegistry, Engine, EngineMode, EntityTypeRegistry } from "../../pkg/client.js";

enum LeftBarTab {
  Blocks,
  Entities,
}

export default function LeftBar({
  engine,
  blockRegistry,
  entityTypeRegistry,
  setSelectedEntity,
}: {
  engine: Engine;
  currentMode: EngineMode;
  blockRegistry: BlockRegistry;
  entityTypeRegistry: EntityTypeRegistry;
  setSelectedEntity: (bool) => void;
}) {
  const [currentTab, setCurrentTab] = useState(LeftBarTab.Blocks);
  let theContent;
  if (currentTab === LeftBarTab.Blocks) {
    theContent = (
      <BlockList
        blockRegistry={blockRegistry}
        setEngineBlockIndex={(idx) => {
          engine.ctx_set_editor_block_id(idx);
        }}
      />
    );
  } else if (currentTab === LeftBarTab.Entities) {
    theContent = (
      <EntityTypeList
        entityTypeRegistry={entityTypeRegistry}
        setEngineEntityIndex={(idx) => {
          engine.ctx_set_editor_entity_type_id(idx);
        }}
      />
    );
  }

  // TODO: If we ever need to use it for anything else, this tab-bar business
  // can sensibly be separated into its own component
  return (
    <div className="editor-panel editor-only" id="toolbox">
      <div className="tab-bar">
        <button
          className={currentTab == LeftBarTab.Blocks ? "tab-on" : ""}
          onClick={() => {
            setCurrentTab(LeftBarTab.Blocks);
          }}
        >
          Blocks
        </button>
        <button
          className={currentTab == LeftBarTab.Entities ? "tab-on" : ""}
          onClick={() => {
            setCurrentTab(LeftBarTab.Entities);
          }}
        >
          Entities
        </button>
      </div>
      {theContent}
    </div>
  );
}
