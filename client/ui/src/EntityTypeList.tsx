// A list of EntityTypes
import { EntityTypeRegistry, EntityType } from "../../pkg/client.js";
import { useState } from "react";

export default function EntityTypeList({ entityTypeRegistry, setEngineEntityIndex }: { entityTypeRegistry: EntityTypeRegistry, setEngineEntityIndex: (number: number) => void }) {
    const entityTypes = Array.from(entityTypeRegistry.entity_types);
    const [selectedEntityIndex, setSelectedEntityIndexState] = useState(0);

    if (!entityTypes) {
        return <p>Loading entity types...</p>;
    }

    const setSelectedEntityType = (index: number) => {
        setEngineEntityIndex(index);
        setSelectedEntityIndexState(index);
    };

    return <div className="entity-button-container">
        {entityTypes.map((entityType: EntityType) => {
            let isOn = entityType.id == selectedEntityIndex;
            // TODO: at some point these should be real icons
            let imageUrl = "/client/ui/public/entity-generic-flag.png";
            return (
                <button
                    className={"entity-button " + (isOn ? "button-on" : "")}
                    onClick={(_) => { setSelectedEntityType(entityType.id); }}
                >
                    <img src={imageUrl} alt="" width="32" height="32" />{entityType.name}
                </button>
            );
        })}
    </div>;
}
