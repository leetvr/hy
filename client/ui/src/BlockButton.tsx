import { BlockType } from "../../pkg/client.js";

export default function BlockButton({ blockType, onClickHandler, isOn }: { blockType: BlockType | undefined, onClickHandler: (BlockType) => void, isOn: boolean }) {
    let imageUrl;
    let blockName;
    if(blockType === undefined) {
        imageUrl = "/client/ui/public/block-delete.png";
        blockName = "Delete block";
    } else {
        imageUrl = getBlockImageUrlByType(blockType);
        blockName = blockType.name;
    }
    return <button className={"block-button " + (isOn ? "button-on" : "")} onClick={() => { onClickHandler(blockType) }}>
        <img src={imageUrl} alt="" width="32" height="32" /> {blockName}
    </button>;
}

function getBlockImageUrlByType(blockType: BlockType): string {
    // TODO: when there's an actual blockType.image, do it here
    return "/client/ui/public/block-generic.png";
}
