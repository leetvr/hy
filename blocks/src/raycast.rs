use glam::Vec3;

use crate::{BlockGrid, BlockPos, EMPTY_BLOCK};

#[derive(Debug)]
pub struct RayHit {
    pub position: BlockPos,
}

#[derive(Debug, Copy, Clone)]
pub enum RaycastMode {
    // We want to do something with the block that was selected
    Selecting,
    // We want to do something *above* the block that was selected
    Placing,
}

pub(crate) fn raycast(
    blocks: &BlockGrid,
    starting_position: Vec3,
    direction: Vec3,
    floor: f32,
    mode: RaycastMode,
) -> Option<RayHit> {
    // Implementation of Amanatides and Woo's raycasting algorithm

    // Ported from https://web.archive.org/web/20121024081332/www.xnawiki.com/index.php?title=Voxel_traversal

    let ray_direction = direction.normalize();
    let ray_dir_zero = ray_direction.cmpeq(Vec3::ZERO);

    let mut current_voxel = starting_position.floor();

    // Determine which way we go.
    let step = ray_direction.signum();

    // Calculate cell boundaries. When the step (i.e. direction sign) is positive,
    // the next boundary is AFTER our current position, meaning that we have to add 1.
    // Otherwise, it is BEFORE our current position, in which case we add nothing.
    let cell_boundary = current_voxel + Vec3::select(step.cmpgt(Vec3::ZERO), Vec3::ONE, Vec3::ZERO);

    // Determine how far we can travel along the ray before we hit a voxel boundary.
    let t_max = (cell_boundary - starting_position) / ray_direction;
    // Sanitize the NaNs
    let mut t_max = Vec3::select(ray_dir_zero, Vec3::INFINITY, t_max);

    // Determine how far we must travel along the ray before we have crossed a gridcell.
    let t_delta = step / ray_direction;
    // Sanitize the NaNs
    let t_delta = Vec3::select(ray_dir_zero, Vec3::INFINITY, t_delta);

    let (_, max_y, _) = blocks.size();

    for _ in 0..1000 {
        let blockpos = BlockPos::from_signed(current_voxel);
        let block = blockpos.and_then(|pos| blocks.get(pos)).copied();
        let block_is_empty = block.map_or(true, |block| block == EMPTY_BLOCK);

        if !block_is_empty {
            let mut blockpos = blockpos.unwrap();

            // If we're in placing mode, we actually want the blockpos *above* this one.
            match mode {
                RaycastMode::Placing => {
                    if blockpos.y < max_y as i32 {
                        blockpos.y += 1;
                    }
                }
                _ => {}
            };
            return Some(RayHit { position: blockpos });
        }

        // If we hit below the floor, we return the block above the floor.
        let below_floor = current_voxel.y < floor;
        if below_floor {
            let blockpos = BlockPos::from_signed(current_voxel + Vec3::Y)?;
            return Some(RayHit { position: blockpos });
        }

        let min_element_of_t_max = t_max.min_element();

        // X is the lowest of the t_max values. A YZ voxel boundary is nearest.
        if t_max.x == min_element_of_t_max {
            current_voxel.x += step.x;
            t_max.x += t_delta.x;
        }
        // Y is the lowest of the t_max values. A XZ voxel boundary is nearest.
        else if t_max.y == min_element_of_t_max {
            current_voxel.y += step.y;
            t_max.y += t_delta.y;
        }
        // Z is the lowest of the t_max values. A XY voxel boundary is nearest.
        else {
            debug_assert_eq!(t_max.z, min_element_of_t_max);
            current_voxel.z += step.z;
            t_max.z += t_delta.z;
        }
    }
    None
}
