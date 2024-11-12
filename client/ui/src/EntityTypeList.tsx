// A list of Entitys
import { EntityTypeRegistry, EntityType } from "../../pkg/client.js";

export default function EntityTypeList({ entityTypeRegistry, setEngineEntityIndex }: { entityTypeRegistry: EntityTypeRegistry, setEngineEntityIndex: (number: number) => void }) {
    const entityTypes = Array.from(entityTypeRegistry.entity_types);

    if (!entityTypes) {
        return <p>Loading entity types...</p>;
    }

    const setSelectedEntityType = (index: number) => {
        setEngineEntityIndex(index);
    };

    return <div className="entity-button-container">
        {entityTypes.map((entityType: EntityType) => {
            return (
                <button onClick={(_) => { setSelectedEntityType(entityType.id); }}>{entityType.name}</button>
            );
        })}
    </div>;
}
