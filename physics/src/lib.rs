use {
    nalgebra::{point, Vector3},
    rapier3d::{
        dynamics::RigidBodyHandle,
        geometry::{Group, InteractionGroups},
        math::{Point, Vector},
        na::vector,
        parry::query::ShapeCastOptions,
        pipeline::QueryFilter,
        prelude::{
            CCDSolver, ColliderBuilder, ColliderHandle, ColliderSet, DebugRenderBackend,
            DebugRenderMode, DebugRenderObject, DebugRenderPipeline, DefaultBroadPhase,
            ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase,
            PhysicsPipeline, QueryPipeline, Real, RigidBodyBuilder, RigidBodySet,
        },
    },
    std::{
        collections::{HashMap, HashSet},
        f32::EPSILON,
    },
};

const TERRAIN_GROUP: Group = Group::GROUP_1;
const PLAYER_GROUP: Group = Group::GROUP_2;
const GRAVITY: f32 = -0.20;
pub const TICK_RATE: u32 = 60;
pub const TICK_DT: f32 = 1. / TICK_RATE as f32;

pub struct PhysicsWorld {
    // Parameters
    gravity: Vector<f32>,

    // Physics logic things
    physics_pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    physics_hooks: (),
    event_handler: (),

    // Physics objects
    islands: IslandManager,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    pub player_handles: HashMap<u64, RigidBodyHandle>,
    debug: DebugRenderPipeline,
    debug_lines: Vec<net_types::DebugLine>,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        // Parameters
        let gravity = vector![0.0, -30., 0.0];
        let integration_parameters = IntegrationParameters::default();

        // Engine
        let physics_pipeline = PhysicsPipeline::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let ccd_solver = CCDSolver::new();
        let query_pipeline = QueryPipeline::new();
        let physics_hooks = ();
        let event_handler = ();

        // Physics object sets
        let islands = IslandManager::new();
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();

        PhysicsWorld {
            gravity,
            physics_pipeline,
            integration_parameters,
            islands,
            broad_phase,
            narrow_phase,
            bodies,
            colliders,
            impulse_joints,
            multibody_joints,
            ccd_solver,
            query_pipeline,
            physics_hooks,
            event_handler,
            player_handles: Default::default(),
            debug: DebugRenderPipeline::new(Default::default(), DebugRenderMode::COLLIDER_SHAPES),
            debug_lines: Default::default(),
        }
    }

    pub fn step(&mut self) {
        self.debug_lines.clear();
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &self.physics_hooks,
            &self.event_handler,
        );
    }

    /// Adds a ball rigidbody
    pub fn add_ball_body(&mut self, position: glam::Vec3, size: f32) -> PhysicsBody {
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(vector![position.x, position.y, position.z])
            .ccd_enabled(true)
            .enabled_rotations(false, false, false)
            .build();
        let collider = ColliderBuilder::ball(size).build();
        let handle = self.bodies.insert(rigid_body);
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);

        PhysicsBody {
            handle,
            removed: false,
        }
    }

    /// Add player body
    /// Creates a kinematic body with a cuboid collider
    pub fn add_player_body(
        &mut self,
        player_id: u64, // evil
        position: glam::Vec3,
        player_width: f32,
        player_height: f32,
    ) -> PhysicsBody {
        let rigid_body = RigidBodyBuilder::kinematic_position_based()
            .translation(glam_to_na(position))
            .enabled_rotations(false, false, false)
            .enabled_translations(false, false, false)
            .user_data(player_id as _)
            .ccd_enabled(true)
            .build();
        let half_height = player_height / 2.0;
        let radius = player_width / 2.0;
        tracing::debug!("Creating player with half height of {half_height} and radius {radius}");
        let collider = ColliderBuilder::cylinder(half_height, radius)
            .collision_groups(InteractionGroups::new(PLAYER_GROUP, TERRAIN_GROUP))
            .position(vector![0.0, half_height, 0.0].into())
            .build();
        let handle = self.bodies.insert(rigid_body);
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);

        self.player_handles.insert(player_id, handle);

        PhysicsBody {
            handle,
            removed: false,
        }
    }

    /// Removes a rigidbody
    pub fn remove_body(&mut self, mut body: PhysicsBody) {
        // Remove the body from the physics world, also removing attached colliders.
        // We don't expose the collider handle for rigidbodies to the user, less state to juggle
        body.removed = true;
        self.bodies.remove(
            body.handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    /// Apply an impulse to a rigidbody
    pub fn apply_impulse(&mut self, body: &PhysicsBody, impulse: glam::Vec3) {
        let rigid_body = &mut self.bodies[body.handle];
        rigid_body.apply_impulse(vector![impulse.x, impulse.y, impulse.z], true);
    }

    /// Set the velocity of a rigidbody
    ///
    /// If a component is None, the velocity in that direction is not changed
    pub fn set_velocity_piecewise(
        &mut self,
        body: &PhysicsBody,
        x: Option<f32>,
        y: Option<f32>,
        z: Option<f32>,
    ) {
        let rigid_body = &mut self.bodies[body.handle];
        let velocity = rigid_body.linvel();
        rigid_body.set_linvel(
            vector![
                x.unwrap_or(velocity.x),
                y.unwrap_or(velocity.y),
                z.unwrap_or(velocity.z)
            ],
            true,
        );
    }

    /// Set the velocity and position of a rigid body
    pub fn set_velocity_and_position(
        &mut self,
        body: &PhysicsBody,
        velocity: glam::Vec3,
        position: glam::Vec3,
    ) {
        let Some(rigid_body) = self.bodies.get_mut(body.handle) else {
            tracing::error!("Rigid body not found! Refusing to update");
            return;
        };
        rigid_body.set_linvel(vector![velocity.x, velocity.y, velocity.z,], true);
        rigid_body.set_position(vector![position.x, position.y, position.z,].into(), true);
    }

    /// Get the position of a rigidbody
    pub fn get_position(&self, body: &PhysicsBody) -> glam::Vec3 {
        let rigid_body = &self.bodies[body.handle];
        glam::Vec3::new(
            rigid_body.translation().x,
            rigid_body.translation().y,
            rigid_body.translation().z,
        )
    }

    /// Adds a trimesh collider
    pub fn add_trimesh_collider(
        &mut self,
        vertices: impl Iterator<Item = glam::Vec3>,
        indices: impl Iterator<Item = [u32; 3]>,
    ) -> PhysicsCollider {
        let vertices: Vec<_> = vertices.map(|v| point![v.x, v.y, v.z]).collect();
        let indices: Vec<_> = indices.collect();
        let collider = ColliderBuilder::trimesh(vertices, indices)
            .collision_groups(InteractionGroups::new(TERRAIN_GROUP, Group::all()))
            .position(vector![0.0, 0.0, 0.0].into())
            .build();
        let handle = self.colliders.insert(collider);
        PhysicsCollider {
            handle,
            removed: false,
        }
    }

    /// Adds a cuboid static collider
    pub fn add_cuboid_collider(
        &mut self,
        position: glam::Vec3,
        size: glam::Vec3,
    ) -> PhysicsCollider {
        let collider = ColliderBuilder::cuboid(size.x, size.y, size.z)
            .translation(vector![position.x, position.y, position.z])
            .build();
        let handle = self.colliders.insert(collider);
        PhysicsCollider {
            handle,
            removed: false,
        }
    }

    /// Removes a collider
    pub fn remove_collider(&mut self, mut collider: PhysicsCollider) {
        collider.removed = true;
        self.colliders.remove(
            collider.handle,
            &mut self.islands,
            &mut self.bodies,
            false, // There shouldn't be any rigidbody attached to this collider
        );
    }

    /// Checks if a player is standing on the ground
    pub fn is_player_on_ground(&mut self, player_id: u64) -> bool {
        // Extract the player's shape
        let Some(player_body_handle) = self.player_handles.get(&player_id) else {
            tracing::warn!("Couldn't find a player handle for {player_id}");
            return false;
        };
        let player_body_handle = *player_body_handle;
        let player_body = &self.bodies[player_body_handle];
        let Some(player_collider_handle) = player_body.colliders().first() else {
            tracing::warn!("Couldn't find a collider for {player_id}");
            return false;
        };

        let player_collider = &self.colliders[*player_collider_handle];
        let current_position = player_collider.position();
        let shape = self.colliders.get(*player_collider_handle).unwrap().shape();

        // Ground detection
        let down_direction = -Vector3::y_axis();
        let mut options = ShapeCastOptions::default();
        options.max_time_of_impact = 0.1;
        options.stop_at_penetration = true;

        if let Some((collided, hit)) = self.query_pipeline.cast_shape(
            &self.bodies,
            &self.colliders,
            &current_position,
            &down_direction,
            shape,
            options,
            QueryFilter::default().groups(InteractionGroups::new(PLAYER_GROUP, TERRAIN_GROUP)),
        ) {
            let collided = &self.colliders[collided];
            let collided_position = collided.position().translation;
            tracing::trace!(
                "Player is on ground: {current_position:?}, {collided_position:?}, toi: {}",
                hit.time_of_impact
            );

            true
        } else {
            tracing::trace!("Player is not on the ground: {current_position:?}");
            false
        }
    }

    pub fn check_movement_for_collisions(
        &mut self,
        player_id: u64,
        movement: glam::Vec3,
    ) -> Option<glam::Vec3> {
        // Extract the player's shape
        let Some(player_body_handle) = self.player_handles.get(&player_id) else {
            tracing::warn!("Couldn't find a player handle for {player_id}");
            return None;
        };

        let player_body_handle = *player_body_handle;
        let player_body = &self.bodies[player_body_handle];
        let Some(player_collider_handle) = player_body.colliders().first() else {
            tracing::warn!("Couldn't find a collider for {player_id}");
            return None;
        };

        let player_collider = &self.colliders[*player_collider_handle];
        let current_position = player_collider.position();
        let player_shape = player_collider.shape();

        // Collision detection
        let movement = glam_to_na(movement);
        let max_distance = movement.norm();
        let movement_normalized = movement.normalize();
        let options = ShapeCastOptions::with_max_time_of_impact(max_distance);

        let (_, hit) = self.query_pipeline.cast_shape(
            &self.bodies,
            &self.colliders,
            &current_position,
            &movement_normalized,
            player_shape,
            options,
            QueryFilter::default()
                .exclude_rigid_body(player_body_handle)
                .groups(InteractionGroups::new(PLAYER_GROUP, TERRAIN_GROUP)),
        )?;

        // Compute the sliding vector
        let hit_normal = hit.normal2.into_inner();

        // Herpedy-derp, mathematics
        let sliding = movement - hit_normal * movement.dot(&hit_normal);

        // Check for zero vector before normalization
        let adjusted_movement_normalized = if sliding.norm() > EPSILON {
            sliding.normalize()
        } else {
            rapier3d::na::Vector3::zeros()
        };

        // Move the character up to the collision point minus a small epsilon
        let movement_distance = (hit.time_of_impact - 0.001).max(0.0);
        let corrected_movement = na_to_glam(adjusted_movement_normalized * movement_distance);

        Some(corrected_movement)
    }

    pub fn get_debug_lines(&mut self) -> Vec<net_types::DebugLine> {
        let mut lines = Vec::new();
        let colliders = self
            .player_handles
            .values()
            .filter_map(|handle| {
                let body = &self.bodies.get(*handle)?;
                let handle = body.colliders().first()?;
                Some(handle.clone())
            })
            .collect::<HashSet<_>>();

        let mut backend = PhysicsRenderer {
            lines: &mut lines,
            colliders,
        };
        self.debug.render(
            &mut backend,
            &self.bodies,
            &self.colliders,
            &self.impulse_joints,
            &self.multibody_joints,
            &self.narrow_phase,
        );

        lines.append(&mut self.debug_lines);

        lines
    }
}

fn na_to_glam(input: nalgebra::Vector3<f32>) -> glam::Vec3 {
    glam::vec3(input.x, input.y, input.z)
}

fn na_point_to_glam(input: Point<f32>) -> glam::Vec3 {
    glam::vec3(input.x, input.y, input.z)
}

/// A handle to a rigidbody in the physics world, with a collider attached
#[derive(Debug)]
pub struct PhysicsBody {
    handle: RigidBodyHandle,
    // Keep track of whether this hnadle has been removed from the physics world to warn the user
    // if a handle is dropped without being removed
    removed: bool,
}

impl Drop for PhysicsBody {
    fn drop(&mut self) {
        if !self.removed {
            let backtrace = std::backtrace::Backtrace::capture();
            tracing::warn!(
                "PhysicsBody dropped without being removed from the physics world: {backtrace}"
            );
        }
    }
}

/// A handle to a collider in the physics world, without any attached rigidbody
#[derive(Debug)]
pub struct PhysicsCollider {
    handle: ColliderHandle,
    // Keep track of whether this hnadle has been removed from the physics world to warn the user
    // if a handle is dropped without being removed
    removed: bool,
}

impl Drop for PhysicsCollider {
    fn drop(&mut self) {
        if !self.removed {
            tracing::warn!("PhysicsCollider dropped without being removed from the physics world");
        }
    }
}

fn glam_to_na(input: glam::Vec3) -> nalgebra::Vector3<f32> {
    vector![input.x, input.y, input.z]
}
struct PhysicsRenderer<'a> {
    lines: &'a mut Vec<net_types::DebugLine>,
    /// Colliders to include
    colliders: HashSet<ColliderHandle>,
}

impl<'a> DebugRenderBackend for PhysicsRenderer<'a> {
    fn draw_line(
        &mut self,
        object: DebugRenderObject,
        a: Point<Real>,
        b: Point<Real>,
        _color: [f32; 4],
    ) {
        match object {
            DebugRenderObject::Collider(handle, _) => {
                if !self.colliders.contains(&handle) {
                    return;
                }
            }
            _ => return,
        };

        self.lines.push(net_types::DebugLine::new(
            na_point_to_glam(a),
            na_point_to_glam(b),
        ));
    }
}
