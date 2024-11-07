import { useState } from "react";
import { Engine } from "../../pkg/client";

export default function Editor({ engine }: { engine: Engine }) {
  const [blockID, setBlockID] = useState(0);

  const switchBlockId = (id: number) => {
    setBlockID(id);
    engine.ctx_set_editor_block_id(id);
  };

  return (
    <>
      <p>Placing block ID {blockID}</p>
      <button onClick={() => switchBlockId(0)}>Use block ID 0</button>
      <button onClick={() => switchBlockId(1)}>Use block ID 1</button>
    </>
  );
}
