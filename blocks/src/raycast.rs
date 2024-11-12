use glam::Vec3;

use crate::{BlockGrid, BlockPos, EMPTY_BLOCK};

#[derive(Debug)]
pub struct RayHit {
    pub position: BlockPos,
    /// The normal of the face we entered the given block through.
    pub entrance_face_normal: Vec3,
}

pub(crate) fn raycast(
    blocks: &BlockGrid,
    starting_position: Vec3,
    direction: Vec3,
    floor: f32,
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

    // This is the normal of the face we just _entered_ through.
    let mut entrance_face_normal = Vec3::ZERO;

    for _ in 0..1000 {
        let blockpos = BlockPos::from_float(current_voxel);
        let block = blockpos.and_then(|pos| blocks.get(pos)).copied();
        let block_is_empty = block.map_or(true, |block| block == EMPTY_BLOCK);

        if !block_is_empty {
            let blockpos = blockpos.unwrap();

            return Some(RayHit {
                position: blockpos,
                entrance_face_normal,
            });
        }

        // If we hit below the floor, we return the block above the floor.
        let below_floor = current_voxel.y < floor;
        if below_floor {
            let blockpos = BlockPos::from_float(current_voxel + Vec3::Y)?;

            // If we're out of bounds, we return None.
            if blocks.get(blockpos).is_none() {
                return None;
            }

            return Some(RayHit {
                position: blockpos,
                entrance_face_normal,
            });
        }

        let min_element_of_t_max = t_max.min_element();

        // X is the lowest of the t_max values. A YZ voxel boundary is nearest.
        if t_max.x == min_element_of_t_max {
            current_voxel.x += step.x;
            t_max.x += t_delta.x;
            entrance_face_normal = Vec3::new(-step.x, 0.0, 0.0);
        }
        // Y is the lowest of the t_max values. A XZ voxel boundary is nearest.
        else if t_max.y == min_element_of_t_max {
            current_voxel.y += step.y;
            t_max.y += t_delta.y;
            entrance_face_normal = Vec3::new(0.0, -step.y, 0.0);
        }
        // Z is the lowest of the t_max values. A XY voxel boundary is nearest.
        else {
            debug_assert_eq!(t_max.z, min_element_of_t_max);
            current_voxel.z += step.z;
            t_max.z += t_delta.z;
            entrance_face_normal = Vec3::new(0.0, 0.0, -step.z);
        }
    }
    None
}
