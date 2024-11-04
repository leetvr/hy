///
/// This crate contains raw data types and utilities for working with blocks in the game world
///
/// This should be used by both the server and the client, and not contain any game logic
///
use {
    serde::{Deserialize, Serialize},
    std::ops::{Index, IndexMut},
};

pub type BlockId = u8;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl From<[u32; 3]> for BlockPos {
    fn from([x, y, z]: [u32; 3]) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockGrid {
    blocks: Vec<BlockId>,
    // X, Y, Z
    size: (u32, u32, u32),
}

impl BlockGrid {
    pub fn new(size_x: u32, size_y: u32, size_z: u32) -> Self {
        Self {
            blocks: vec![BlockId::default(); (size_x * size_y * size_z) as usize],
            size: (size_x, size_y, size_z),
        }
    }

    pub fn get(&self, pos: BlockPos) -> Option<&BlockId> {
        self.blocks.get(block_pos_to_array_index(pos, self.size))
    }

    pub fn get_mut(&mut self, pos: BlockPos) -> Option<&mut BlockId> {
        self.blocks
            .get_mut(block_pos_to_array_index(pos, self.size))
    }

    pub fn size(&self) -> (u32, u32, u32) {
        self.size
    }
}

impl Index<BlockPos> for BlockGrid {
    type Output = BlockId;

    fn index(&self, pos: BlockPos) -> &BlockId {
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
    fn index_mut(&mut self, pos: BlockPos) -> &mut BlockId {
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

fn block_pos_to_array_index(pos: BlockPos, size: (u32, u32, u32)) -> usize {
    let (x, y, z) = (pos.x as usize, pos.y as usize, pos.z as usize);
    x + (y * size.0 as usize) + z * (size.0 as usize * size.1 as usize)
}
