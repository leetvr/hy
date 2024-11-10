// The "right bar": invisible unless an entity is selected, and in that latter
// case, the properties panel

// TODO: implement entities and thus this

export default function RightBar({ selectedEntity }: { selectedEntity: boolean } ) {
    if(selectedEntity) {
        return <div className="editor-panel editor-only" id="propbox">
            <p>Siege chopper, checking in 🚁</p>
        </div>;
    } else {
        return <></>;
    }
}
