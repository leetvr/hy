import { BlockType } from "../../pkg/client.js";

export default function BlockButton({ blockType, onClickHandler, isOn }: { blockType: BlockType | undefined, onClickHandler: (BlockType) => void, isOn: boolean }) {
    let blockImg;
    let blockName;
    if(blockType === undefined) {
        blockName = "Delete block";
        blockImg = <img src="/client/ui/public/block-delete.png" alt="" width="32" height="32" />;
    } else {
        blockName = blockType.name;
        blockImg = <div className="block-cube-ctr">
            <div className="block-cube-cube">
                <div className="cube-face top" style={ {'background-image': 'url("' + blockType.top_texture + '")'} }></div>
                <div className="cube-face left" style={ {'background-image': 'url("' + blockType.south_texture + '")'} }></div>
                <div className="cube-face right" style={ {'background-image': 'url("' + blockType.east_texture + '")'} }></div>
            </div>
        </div>;
    }
    return <button className={"block-button " + (isOn ? "button-on" : "")} onClick={() => { onClickHandler(blockType) }}>
        {blockImg} {blockName}
    </button>;
}
