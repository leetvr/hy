// A list of blocks
import BlockButton from "./BlockButton.tsx";
import { BlockRegistry } from "../../pkg/client.js";

export default function BlockList({ blockRegistry }: { BlockRegistry }) {
    const blockTypes = Array.from(blockRegistry.block_types);
                    //<button onClick={() => switchBlockId(id + 1)}>Use {blockType.name}</button>;
    return <><div className="block-button-container">
        <BlockButton onClickHander={() => {}} />
        {blockRegistry.block_types.map((blockType, index) => {
            return (
                <BlockButton blockType={blockType} onClickHander={() => {}} />
            );
        })}
        </div>
    </>;
}
