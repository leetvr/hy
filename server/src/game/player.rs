use {
    crate::game::PlayerCollision,
    blocks::{BlockGrid, BlockPos, EMPTY_BLOCK},
};

pub fn player_aabb_block_collisions(
    position: glam::Vec3,
    blocks: &BlockGrid,
) -> Vec<PlayerCollision> {
    let player_aabb = aabb_for_player(position);

    let mut collisions = Vec::new();
    for BlockPos { x, y, z } in aabb_colliding_blocks(blocks, &player_aabb) {
        // Collect the 6 possible collisions and the way to resolve getting out of the block
        collisions.extend(
            [
                (glam::Vec3::X, player_aabb.min.x - (x + 1) as f32),
                (-glam::Vec3::X, x as f32 - player_aabb.max.x),
                (glam::Vec3::Y, player_aabb.min.y - (y + 1) as f32),
                (-glam::Vec3::Y, y as f32 - player_aabb.max.y),
                (glam::Vec3::Z, player_aabb.min.z - (z + 1) as f32),
                (-glam::Vec3::Z, z as f32 - player_aabb.max.z),
            ]
            .into_iter()
            .map(|(normal, dist)| PlayerCollision {
                block: BlockPos::new(x, y, z),
                normal,
                resolution: normal * dist.abs() * 1.1,
            })
            .filter(|collision| {
                // Filter out collisions where the resolution would make the player
                // collide even more. We want less collisions, not more.
                if aabb_colliding_blocks(blocks, &aabb_for_player(position + collision.resolution))
                    .next()
                    .is_none()
                {
                    true
                } else {
                    false
                }
            }),
        );
    }
    collisions
}

fn aabb_colliding_blocks<'a>(
    blocks: &'a BlockGrid,
    aabb: &'a AABB,
) -> impl Iterator<Item = BlockPos> + 'a {
    let min = aabb.min_block_pos();
    let max = aabb.max_block_pos();

    // Broad phase, all the blocks we could possibly collide with
    let collidable_blocks = (min.x..=max.x).flat_map(move |x| {
        (min.y..=max.y).flat_map(move |y| {
            (min.z..=max.z).filter_map(move |z| {
                let pos = BlockPos::new(x, y, z);
                if blocks
                    .get(BlockPos::new(x, y, z))
                    .cloned()
                    .map(|b| b != EMPTY_BLOCK)
                    // Out of bounds is solid
                    .unwrap_or(true)
                {
                    Some(pos)
                } else {
                    None
                }
            })
        })
    });

    // Narrow phase, only the blocks we actually collide with
    collidable_blocks.filter(|pos| {
        if aabb.min.x > pos.x as f32 + 1.
            || aabb.max.x < pos.x as f32
            || aabb.min.y > pos.y as f32 + 1.
            || aabb.max.y < pos.y as f32
            || aabb.min.z > pos.z as f32 + 1.
            || aabb.max.z < pos.z as f32
        {
            false
        } else {
            true
        }
    })
}

fn aabb_for_player(player_pos: glam::Vec3) -> AABB {
    let size = glam::Vec3::new(0.5, 1.5, 0.5);
    let min = player_pos - size / 2.;
    let max = player_pos + size / 2.;
    AABB { min, max }
}

struct AABB {
    min: glam::Vec3,
    max: glam::Vec3,
}

impl AABB {
    fn min_block_pos(&self) -> BlockPos {
        BlockPos::new(
            self.min.x.floor() as u32,
            self.min.y.floor() as u32,
            self.min.z.floor() as u32,
        )
    }

    fn max_block_pos(&self) -> BlockPos {
        BlockPos::new(
            self.max.x.floor() as u32,
            self.max.y.floor() as u32,
            self.max.z.floor() as u32,
        )
    }
}
