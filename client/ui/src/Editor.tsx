import { useState } from "react";
import { BlockRegistry, Engine } from "../../pkg/client";

export default function Editor({
  engine,
  blockRegistry,
}: {
  engine: Engine;
  blockRegistry: BlockRegistry | undefined;
}) {
  const [selectedBlockID, setSelectedBlockID] = useState(0);
  const [selectedBlockName, setSelectedBlockName] = useState("Empty Block");

  const switchBlockId = (blockID: number) => {
    setSelectedBlockID(blockID);
    engine.ctx_set_editor_block_id(blockID);

    if (blockID == 0) {
      setSelectedBlockName("Empty Block");
    } else {
      const index = blockID - 1;
      const blockType = blockRegistry!.block_types[index]; // safe since this can't be called unless blockRegistry exists
      const name = blockType.name;
      console.log("blockID", blockID, "index", index, "name", name);
      setSelectedBlockName(name);
    }
  };

  if (!blockRegistry) {
    return <p>Loading blocks..</p>;
  }

  return (
    <>
      <p>
        Placing {selectedBlockName} (id: {selectedBlockID})
      </p>
      <div>
        <button onClick={() => switchBlockId(0)}>Set empty block</button>
      </div>
      {blockRegistry.block_types.map((blockType, id) => {
        return (
          <div>
            <button onClick={() => switchBlockId(id + 1)}>Use {blockType.name}</button>;
          </div>
        );
      })}
    </>
  );
}
