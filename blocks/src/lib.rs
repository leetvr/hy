///
/// This crate contains raw data types and utilities for working with blocks in the game world
///
/// This should be used by both the server and the client, and not contain any game logic
///
use {
    serde::{Deserialize, Serialize},
    std::ops::{Add, Index, IndexMut, Sub},
};

use glam::{IVec3, UVec3, Vec3};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

mod raycast;

pub use raycast::RayHit;

pub type BlockTypeID = u8;

pub const EMPTY_BLOCK: BlockTypeID = 0;
pub const MAX_BLOCK_HEIGHT: u32 = 64;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl BlockPos {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    pub fn from_float(vec: Vec3) -> Option<Self> {
        if vec.cmplt(Vec3::ZERO).any() {
            return None;
        }

        Some(Self {
            x: vec.x as u32,
            y: vec.y as u32,
            z: vec.z as u32,
        })
    }

    pub fn add_signed(&self, vec: IVec3) -> Option<Self> {
        let result = UVec3::new(self.x, self.y, self.z).as_ivec3() + vec;

        if result.cmplt(IVec3::ZERO).any() {
            return None;
        }

        Some(Self {
            x: result.x as u32,
            y: result.y as u32,
            z: result.z as u32,
        })
    }
}

impl Into<glam::Vec3> for BlockPos {
    fn into(self) -> glam::Vec3 {
        glam::Vec3::new(self.x as f32, self.y as f32, self.z as f32)
    }
}

impl Sub<BlockPos> for BlockPos {
    type Output = BlockPos;

    fn sub(self, rhs: BlockPos) -> Self::Output {
        BlockPos {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Add<BlockPos> for BlockPos {
    type Output = BlockPos;

    fn add(self, rhs: BlockPos) -> Self::Output {
        BlockPos {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl From<[u32; 3]> for BlockPos {
    fn from([x, y, z]: [u32; 3]) -> Self {
        Self { x, y, z }
    }
}

impl From<UVec3> for BlockPos {
    fn from(vec: UVec3) -> Self {
        Self {
            x: vec.x,
            y: vec.y,
            z: vec.z,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockGrid {
    blocks: Vec<BlockTypeID>,
    // X, Y, Z
    size: (u32, u32, u32),
}

impl BlockGrid {
    /// Create a new block grid with the given dimensions
    pub fn new(size_x: u32, size_y: u32, size_z: u32) -> Self {
        Self {
            blocks: vec![BlockTypeID::default(); (size_x * size_y * size_z) as usize],
            size: (size_x, size_y, size_z),
        }
    }

    /// Get the block at the given position
    pub fn get(&self, pos: BlockPos) -> Option<&BlockTypeID> {
        self.blocks.get(block_pos_to_array_index(pos, self.size)?)
    }

    /// Get a mutable reference to the block at the given position
    pub fn get_mut(&mut self, pos: BlockPos) -> Option<&mut BlockTypeID> {
        self.blocks
            .get_mut(block_pos_to_array_index(pos, self.size)?)
    }

    /// Get the size of the block grid
    pub fn size(&self) -> (u32, u32, u32) {
        self.size
    }

    /// Get an iterator over all non-empty blocks in the grid
    pub fn iter_non_empty(&self) -> impl Iterator<Item = (BlockPos, BlockTypeID)> + '_ {
        let indices = (0..self.size.0).flat_map(move |x| {
            (0..self.size.1).flat_map(move |y| (0..self.size.2).map(move |z| [x, y, z]))
        });
        indices.filter_map(|pos| {
            let block_pos: BlockPos = pos.into();
            self.get(block_pos).and_then(|&block| {
                if block != EMPTY_BLOCK {
                    Some((block_pos, block))
                } else {
                    None
                }
            })
        })
    }

    pub fn raycast(&self, start: Vec3, direction: glam::Vec3) -> Option<raycast::RayHit> {
        raycast::raycast(self, start, direction, 0.0)
    }
}

impl Index<BlockPos> for BlockGrid {
    type Output = BlockTypeID;

    fn index(&self, pos: BlockPos) -> &BlockTypeID {
        self.get(pos)
            .ok_or_else(|| {
                format!(
                    "Index ({}, {}, {}) out of bounds ({}, {}, {})",
                    pos.x, pos.y, pos.z, self.size.0, self.size.1, self.size.2
                )
            })
            .unwrap()
    }
}

impl IndexMut<BlockPos> for BlockGrid {
    fn index_mut(&mut self, pos: BlockPos) -> &mut BlockTypeID {
        let size = self.size;
        self.get_mut(pos)
            .ok_or_else(|| {
                format!(
                    "Index ({}, {}, {}) out of bounds ({}, {}, {})",
                    pos.x, pos.y, pos.z, size.0, size.1, size.2
                )
            })
            .unwrap()
    }
}

fn block_pos_to_array_index(pos: BlockPos, size: (u32, u32, u32)) -> Option<usize> {
    if pos.x >= size.0 || pos.y >= size.1 || pos.z >= size.2 {
        return None;
    }

    let (x, y, z) = (pos.x as usize, pos.y as usize, pos.z as usize);
    Some(x + (y * size.0 as usize) + z * (size.0 as usize * size.1 as usize))
}

#[derive(Tsify, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockType {
    pub name: String,
    pub north_texture: String,
    pub south_texture: String,
    pub east_texture: String,
    pub west_texture: String,
    pub top_texture: String,
    pub bottom_texture: String,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}

#[derive(Tsify, Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct BlockRegistry {
    block_types: Vec<BlockType>,
}

impl BlockRegistry {
    pub fn blocks(&self) -> Vec<BlockType> {
        self.block_types.clone()
    }
}

impl BlockRegistry {
    pub fn get(&self, block_id: BlockTypeID) -> Option<&BlockType> {
        if block_id == EMPTY_BLOCK {
            return None;
        };

        // note(KMRW):
        // We need to subtract 1 here to avoid having to store the empty block which has ID 0.
        // This may be a dumb idea.
        let index = block_id as usize - 1;
        self.block_types.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockType> {
        self.block_types.iter()
    }

    #[cfg(not(target_arch = "wasm32"))]
    // The client should *never* be able to mutate the block registry.
    pub fn insert(&mut self, block_type: BlockType) -> BlockTypeID {
        self.block_types.push(block_type);

        // note(KMRW):
        // We check the length of `block_types` *after* we insert the block to avoid having to
        // store an empty block.
        // This may be a dumb idea.
        let block_id = self.block_types.len() as BlockTypeID;

        block_id
    }
}
