use {
    anyhow::Result,
    blocks::{BlockGrid, BlockPos, BlockRegistry, EMPTY_BLOCK},
    entities::{EntityData, EntityTypeRegistry},
    glam::Vec3,
    physics::{PhysicsCollider, PhysicsWorld},
    std::{collections::HashMap, mem, path::Path},
};

const BLOCKS_PATH: &str = "blocks.json";
const BLOCK_TYPES_PATH: &str = "block_types.json";
const ENTITIES_PATH: &str = "entities.json";
const ENTITY_TYPES_PATH: &str = "entity_types.json";

pub struct World {
    pub physics_world: PhysicsWorld,
    colliders: Vec<PhysicsCollider>,
    pub blocks: BlockGrid,
    pub block_registry: BlockRegistry,
    pub entities: Vec<EntityData>,
    pub entity_type_registry: EntityTypeRegistry,
}

impl World {
    pub fn load(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let blocks_path = storage_dir.as_ref().join(BLOCKS_PATH);
        let blocks = serde_json::from_slice(&std::fs::read(blocks_path)?)?;

        let block_types_path = storage_dir.as_ref().join(BLOCK_TYPES_PATH);
        let block_registry = serde_json::from_slice(&std::fs::read(&block_types_path)?)?;

        let entities_path = storage_dir.as_ref().join(ENTITIES_PATH);
        let entities = serde_json::from_slice(&std::fs::read(entities_path)?)?;

        let entity_types_path = storage_dir.as_ref().join(ENTITY_TYPES_PATH);
        let entity_type_registry = serde_json::from_slice(&std::fs::read(entity_types_path)?)?;

        let mut physics_world = PhysicsWorld::new();
        let mut colliders = Vec::new();

        bake_terrain_colliders(&mut physics_world, &blocks, &mut colliders);

        Ok(Self {
            physics_world,
            colliders,
            blocks,
            block_registry,
            entities,
            entity_type_registry,
        })
    }

    pub fn save(&mut self, storage_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let blocks_path = storage_dir.as_ref().join(BLOCKS_PATH);
        bake_terrain_colliders(&mut self.physics_world, &self.blocks, &mut self.colliders);
        let blocks = serde_json::to_string(&self.blocks)?;
        std::fs::write(blocks_path, blocks)?;
        Ok(())
    }
}

/// Rebuilds the terrain colliders from the block grid
///
/// This builds trimesh colliders, two for each layer along each axis: X+, X-, Y+, Y-, Z+, Z-
pub fn bake_terrain_colliders(
    physics_world: &mut PhysicsWorld,
    blocks: &BlockGrid,
    colliders: &mut Vec<PhysicsCollider>,
) {
    // Remove old colliders
    for collider in colliders.drain(..) {
        physics_world.remove_collider(collider);
    }

    // Vertices can be shared between many faces, store indices for each unique vertex
    let mut vert_indices = HashMap::new();
    let mut last_vert_index: u32 = 0;

    let mut layer_meshes = Vec::new();
    let size = blocks.size();

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let forward_offset = match axis {
            Axis::X => BlockPos::new(1, 0, 0),
            Axis::Y => BlockPos::new(0, 1, 0),
            Axis::Z => BlockPos::new(0, 0, 1),
        };
        let (layers, rows, cols) = match axis {
            Axis::X => (size.0, size.1, size.2),
            Axis::Y => (size.1, size.0, size.2),
            Axis::Z => (size.2, size.0, size.1),
        };

        for layer_pos in 0..layers {
            // We generate 2 meshes for each axis, one for the front face and one for the back face
            let mut front_mesh = Vec::new();
            let mut back_mesh = Vec::new();

            for row in 0..rows {
                for col in 0..cols {
                    let mut pos = BlockPos {
                        x: layer_pos as i32,
                        y: row as i32,
                        z: col as i32,
                    };
                    match axis {
                        Axis::X => {}
                        Axis::Y => mem::swap(&mut pos.x, &mut pos.y),
                        Axis::Z => mem::swap(&mut pos.x, &mut pos.z),
                    }

                    if blocks.get(pos).copied().unwrap_or(EMPTY_BLOCK) == EMPTY_BLOCK {
                        // Empty blocks have no collider
                        continue;
                    }

                    // Block has a collider in the front if there is no block in front of it
                    let front_block = if layer_pos + 1 < layers {
                        blocks
                            .get(pos + forward_offset)
                            .copied()
                            .unwrap_or(EMPTY_BLOCK)
                    } else {
                        EMPTY_BLOCK
                    };
                    if front_block == EMPTY_BLOCK {
                        let vert_indices = axis_face_vertices(axis).map(|vertex| {
                            let vertex_pos = pos + vertex + forward_offset;

                            // Get the unique index for this vertex
                            *vert_indices.entry(vertex_pos).or_insert_with(|| {
                                let i = last_vert_index;
                                last_vert_index += 1;
                                i
                            })
                        });

                        // Add face to mesh, 2 triangles
                        front_mesh.push([vert_indices[0], vert_indices[1], vert_indices[2]]);
                        front_mesh.push([vert_indices[0], vert_indices[2], vert_indices[3]]);
                    }

                    // Block has a collider in the back if there is no block behind it
                    let back_block = if layer_pos > 0 {
                        blocks
                            .get(pos - forward_offset)
                            .copied()
                            .unwrap_or(EMPTY_BLOCK)
                    } else {
                        EMPTY_BLOCK
                    };
                    if back_block == EMPTY_BLOCK {
                        let vert_indices = axis_face_vertices(axis).map(|vertex| {
                            let vertex_pos = pos + vertex;

                            *vert_indices.entry(vertex_pos).or_insert_with(|| {
                                let i = last_vert_index;
                                last_vert_index += 1;
                                i
                            })
                        });

                        // Add face to mesh, 2 triangles
                        back_mesh.push([vert_indices[0], vert_indices[1], vert_indices[2]]);
                        back_mesh.push([vert_indices[0], vert_indices[2], vert_indices[3]]);
                    }
                }
            }

            if front_mesh.len() > 0 {
                layer_meshes.push(front_mesh);
            }
            if back_mesh.len() > 0 {
                layer_meshes.push(back_mesh);
            }
        }
    }

    // Invert the vertices map, putting the vertices in a vec where the indices correspond to the
    // indices generated for each layer mesh
    let mut vertices = vec![Vec3::ZERO; vert_indices.len() as usize];
    for (vertex, index) in vert_indices {
        vertices[index as usize] = Vec3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32);
    }
    tracing::info!(
        "Generating trimesh colliders from {} unique vertices",
        vertices.len()
    );

    for layer_mesh in layer_meshes {
        let collider =
            physics_world.add_trimesh_collider(vertices.iter().copied(), layer_mesh.into_iter());
        colliders.push(collider);
    }
}

/// Get 4 vertices for a block's face aligned along the specified axis
fn axis_face_vertices(axis: Axis) -> [BlockPos; 4] {
    match axis {
        Axis::X => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(0, 0, 1),
            BlockPos::new(0, 1, 1),
            BlockPos::new(0, 1, 0),
        ],
        Axis::Y => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(0, 0, 1),
            BlockPos::new(1, 0, 1),
            BlockPos::new(1, 0, 0),
        ],
        Axis::Z => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(1, 0, 0),
            BlockPos::new(1, 1, 0),
            BlockPos::new(0, 1, 0),
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}
