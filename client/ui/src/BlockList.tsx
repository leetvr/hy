// A list of blocks
import BlockButton from "./BlockButton.tsx";
import { BlockRegistry } from "../../pkg/client.js";
import { useState } from "react";

export default function BlockList({ blockRegistry, setEngineBlockIndex }: { blockRegistry: BlockRegistry, setEngineBlockIndex: (number: number) => void }) {
    const blockTypes = Array.from(blockRegistry.block_types);

    if (!blockRegistry) {
        return <p>Loading blocks...</p>;
    }

    // IMO this should be a useEffect rather than a useState because the state
    // itself is truly managed by the engine context. Buuuuut... that would be
    // irritating to do.
    const [selectedBlockIndex, setSelectedBlockIndexState] = useState(0);

    const setSelectedBlock = (index: number) => {
        setEngineBlockIndex(index);
        setSelectedBlockIndexState(index);
    };

    return <div className="block-button-container">
        <BlockButton isOn={selectedBlockIndex === 0} onClickHandler={() => { setSelectedBlock(0); }} />
        {blockTypes.map((blockType, index) => {
            return (
                <BlockButton isOn={selectedBlockIndex === 1 + index} blockType={blockType} onClickHandler={(_) => { setSelectedBlock(1 + index); }} />
            );
        })}
    </div>;
}
