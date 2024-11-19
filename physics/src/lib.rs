use blocks::BlockTypeID;
use entities::{EntityData, EntityID, EntityPhysicsProperties, EntityState, EntityTypeRegistry};
use glam::Vec3Swizzles;
use nalgebra::{point, vector, Vector3};
use rapier3d::{
    dynamics::RigidBodyHandle,
    geometry::{Group, InteractionGroups},
    math::{Point, Vector},
    parry::query::ShapeCastOptions,
    pipeline::QueryFilter,
    prelude::{
        ActiveCollisionTypes, CCDSolver, Collider, ColliderBuilder, ColliderHandle, ColliderSet,
        DebugRenderBackend, DebugRenderMode, DebugRenderObject, DebugRenderPipeline,
        DefaultBroadPhase, ImpulseJointSet, IntegrationParameters, IslandManager,
        MultibodyJointSet, NarrowPhase, PhysicsPipeline, QueryPipeline, Real, RigidBody,
        RigidBodyBuilder, RigidBodySet, RigidBodyType,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const BLOCK_GROUP: Group = Group::GROUP_1;
const PLAYER_GROUP: Group = Group::GROUP_2;
const ENTITY_GROUP: Group = Group::GROUP_3;

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
    pub entity_bodies: HashMap<EntityID, PhysicsBody>,
}

impl Drop for PhysicsWorld {
    fn drop(&mut self) {
        // Silence warnings on dropping `PhysicsBody`s
        for (_, body) in self.entity_bodies.drain().collect::<Vec<_>>() {
            self.remove_body(body);
        }
    }
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
            entity_bodies: Default::default(),
        }
    }

    pub fn step(
        &mut self,
        entities: &mut HashMap<EntityID, EntityData>,
        entity_type_registry: &EntityTypeRegistry,
    ) {
        self.debug_lines.clear();

        // 1) Update non-dynamic bodies
        for (_, entity_data) in entities.iter_mut() {
            let entity_type = entity_type_registry
                .get(entity_data.entity_type)
                .expect("Entity type ID doesn't exist?");

            let Some(physics_properties) = entity_type.physics_properties() else {
                continue;
            };

            if !physics_properties.dynamic {
                self.sync_entity(entity_data);
            }
        }

        // 2) Step simulation
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

        // 3) Update dynamic bodies
        for (_, entity_data) in entities.iter_mut() {
            let entity_type = entity_type_registry
                .get(entity_data.entity_type)
                .expect("Entity type ID doesn't exist?");

            let Some(physics_properties) = entity_type.physics_properties() else {
                continue;
            };

            if physics_properties.dynamic {
                self.sync_entity(entity_data);
            }
        }
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
        let radius = player_width / 2.0;
        let half_height = player_height / 2.0;
        tracing::debug!("Creating player with half height of {half_height} and radius {radius}");
        let collider = ColliderBuilder::capsule_y(half_height - radius, radius)
            .collision_groups(InteractionGroups::new(PLAYER_GROUP, Group::all()))
            .position(vector![0.0, half_height, 0.0].into())
            .active_collision_types(ActiveCollisionTypes::all())
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
            .collision_groups(InteractionGroups::new(BLOCK_GROUP, Group::all()))
            .position(vector![0.0, 0.0, 0.0].into())
            .build();
        let handle = self.colliders.insert(collider);
        PhysicsCollider {
            handle,
            removed: false,
        }
    }

    /// Adds a block collider
    pub fn add_block_collider(&mut self, position: glam::Vec3, block_type_id: BlockTypeID) {
        let position = position + glam::Vec3::new(0.5, 0.5, 0.5);
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.5)
            .translation(vector![position.x, position.y, position.z])
            .collision_groups(InteractionGroups::new(BLOCK_GROUP, Group::all()))
            .user_data(block_type_id.into())
            .build();
        self.colliders.insert(collider);
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

    pub fn check_movement_for_collisions(
        &mut self,
        player_id: u64,
        current_position: glam::Vec3,
        desired_velocity: glam::Vec3,
    ) -> CollisionResult {
        let Some((_, player_collider_handle)) = self.get_player_handles(player_id) else {
            return CollisionResult {
                corrected_movement: desired_velocity,
                is_on_ground: false,
                would_have_collided: false,
            };
        };

        let result = check_movement_for_collisions(
            player_collider_handle,
            glam_to_na(desired_velocity),
            &self.query_pipeline,
            &self.bodies,
            &self.colliders,
        );

        self.debug_lines.push(net_types::DebugLine {
            start: current_position,
            end: current_position + result.corrected_movement,
        });

        if desired_velocity.xz().length() > 0. {
            tracing::trace!("Current position: {current_position:?}, desired_velocity: {desired_velocity:?}, result: {result:?}")
        }

        result
    }

    fn get_player_handles(&mut self, player_id: u64) -> Option<(RigidBodyHandle, ColliderHandle)> {
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

        Some((player_body_handle.clone(), player_collider_handle.clone()))
    }

    pub fn get_debug_lines(&mut self) -> Vec<net_types::DebugLine> {
        let mut lines = Vec::new();

        let mut backend = PhysicsRenderer { lines: &mut lines };
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

    pub fn spawn_entity(
        &mut self,
        entity_data: &EntityData,
        entity_type_registry: &EntityTypeRegistry,
    ) {
        // Check to see if there's an existing body
        if let Some(body) = self.entity_bodies.remove(&entity_data.id) {
            tracing::warn!(
                "Attempted to spawn entity {}, but it has an old handle lying around. We'll clean it up, but you should fix this.",
                &entity_data.id
            );
            self.remove_body(body);
        };

        let physics_properties = entity_type_registry
            .get(entity_data.entity_type)
            .unwrap()
            .physics_properties();

        // If this entity has no physics properties, then there's nothing to do
        let Some(physics_properties) = physics_properties else {
            tracing::debug!(
                "Entity {} has no physics properties, doing nothing",
                &entity_data.id
            );
            return;
        };

        let rigid_body =
            build_rigid_body_for_entity(&entity_data.id, physics_properties, &entity_data.state);
        let handle = self.bodies.insert(rigid_body);
        let collider = build_collider_for_entity(&entity_data.id, physics_properties);
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);

        self.entity_bodies
            .insert(entity_data.id.clone(), PhysicsBody::new(handle));

        return;
    }

    pub fn sync_entity(&mut self, entity_data: &mut EntityData) {
        // Extract the physics body
        let Some(body) = self.entity_bodies.get(&entity_data.id) else {
            tracing::warn!("Attempted to sync {} but it has no body!", &entity_data.id);
            return;
        };

        let Some(physics_body) = self.bodies.get_mut(body.handle) else {
            tracing::warn!("Attempted to sync {} but it has no body!", &entity_data.id);
            return;
        };

        // If the entity is dynamic, we set its position from the physics engine
        if physics_body.body_type().is_dynamic() {
            let state = &mut entity_data.state;
            let position = physics_body.position();
            state.position = na_to_glam(position.translation.vector);
            state.rotation = na_quat_to_glam(position.rotation);
            entity_data.state.velocity = na_to_glam(physics_body.linvel().clone());

            // Nothing more to do.
            return;
        }

        // If the entity is *not* dynamic, we set the body's position and velocity from the entity
        physics_body.set_linvel(glam_to_na(entity_data.state.velocity), true);
        physics_body.set_next_kinematic_position(glam_to_na(entity_data.state.velocity).into());

        // I don't know why `set_next_kinematic_position` is not enough, but this fixes #209
        physics_body.set_position(glam_to_na(entity_data.state.position).into(), true);
    }

    pub fn despawn_entity(&mut self, entity_id: &str) {
        let Some(body) = self.entity_bodies.remove(entity_id) else {
            tracing::warn!("Attempted to despawn entity {entity_id} but it has no handles");
            return;
        };

        self.remove_body(body);
    }

    pub fn get_collisions_for_entity(&self, entity_id: &EntityID) -> Vec<Collision> {
        let Some(collider) = self
            .entity_bodies
            .get(entity_id)
            .and_then(|body| self.bodies.get(body.handle))
            .and_then(|body| body.colliders().first().cloned())
        else {
            tracing::warn!("Tried to get collisions for entity {entity_id} but it has no body!");
            return Vec::new();
        };

        self.get_collisions_for_collider(collider)
    }

    pub fn get_collisions_for_player(&self, player_id: u64) -> Vec<Collision> {
        let Some(collider) = self
            .player_handles
            .get(&player_id)
            .and_then(|body| self.bodies.get(*body))
            .and_then(|body| body.colliders().first().cloned())
        else {
            tracing::warn!("Tried to get collisions for entity {player_id} but it has no body!");
            return Vec::new();
        };

        self.get_collisions_for_collider(collider)
    }

    fn get_collisions_for_collider(&self, collider: ColliderHandle) -> Vec<Collision> {
        let mut collisions = Vec::new();

        // Check for contacts
        for contact_pair in self.narrow_phase.contact_pairs_with(collider) {
            if !contact_pair.has_any_active_contact {
                continue;
            }

            // You should see the other guy!
            let other_collider_handle = if contact_pair.collider1 == collider {
                contact_pair.collider2
            } else {
                contact_pair.collider1
            };

            let Some(other_collider) = self.colliders.get(other_collider_handle) else {
                // NOTE(kmrw)
                // This can happen. It seems like Rapier doesn't remove colliders fully until the
                // next step, so if we collide with an entity that was despawned last frame, it
                // won't be in the collider list.
                continue;
            };

            let Some(collision_target) = get_entity_collision_target(other_collider) else {
                continue;
            };

            let target_id = other_collider.user_data.to_string();

            collisions.push(Collision {
                collision_kind: CollisionKind::Contact,
                collision_target,
                target_id,
            });
        }

        // Check for intersections
        for (collider1, collider2, intersected) in
            self.narrow_phase.intersection_pairs_with(collider)
        {
            if !intersected {
                continue;
            }

            let other_collider_handle = if collider1 == collider {
                collider2
            } else {
                collider1
            };

            let Some(other_collider) = self.colliders.get(other_collider_handle) else {
                // NOTE(kmrw)
                // This can happen. It seems like Rapier doesn't remove colliders fully until the
                // next step, so if we collide with an entity that was despawned last frame, it
                // won't be in the collider list.
                continue;
            };

            let Some(collision_target) = get_entity_collision_target(other_collider) else {
                continue;
            };

            let target_id = other_collider.user_data.to_string();

            collisions.push(Collision {
                collision_kind: CollisionKind::Intersection,
                collision_target,
                target_id,
            });
        }

        collisions
    }
}

fn get_entity_collision_target(other_collider: &Collider) -> Option<CollisionTarget> {
    let membership = other_collider.collision_groups().memberships;
    if membership.contains(BLOCK_GROUP) {
        return Some(CollisionTarget::Block);
    }

    if membership.contains(PLAYER_GROUP) {
        return Some(CollisionTarget::Player);
    }

    if membership.contains(ENTITY_GROUP) {
        return Some(CollisionTarget::Entity);
    }

    None
}

fn na_quat_to_glam(rotation: nalgebra::Unit<nalgebra::Quaternion<f32>>) -> glam::Quat {
    // jesus, nalgebra
    glam::Quat::from_array(rotation.into_inner().coords.data.0[0])
}

fn build_rigid_body_for_entity(
    id: &str,
    physics_properties: &EntityPhysicsProperties,
    state: &EntityState,
) -> RigidBody {
    let body_type = if physics_properties.dynamic {
        RigidBodyType::Dynamic
    } else {
        RigidBodyType::KinematicVelocityBased
    };

    let user_data: u128 = id
        .parse()
        .expect("Entity ID is not a number - this should be impossible");
    let builder = RigidBodyBuilder::new(body_type)
        .ccd_enabled(true)
        .user_data(user_data)
        .linvel(glam_to_na(state.velocity))
        .position(glam_to_na(state.position).into());

    tracing::debug!("Built rigid body for {id} with properties {physics_properties:?}");

    // TODO: Add more properties
    builder.build()
}

fn build_collider_for_entity(
    id: &str,
    physics_properties: &entities::EntityPhysicsProperties,
) -> Collider {
    let EntityPhysicsProperties {
        collider_kind,
        collider_height,
        collider_width,
        ..
    } = physics_properties;

    let half_height = collider_height / 2.0;
    let half_width = collider_width / 2.0;
    let builder = match collider_kind {
        entities::EntityColliderKind::Capsule => {
            ColliderBuilder::capsule_y(half_height, half_width)
        }
        entities::EntityColliderKind::Cube => {
            ColliderBuilder::cuboid(half_width, half_width, half_width)
        }
        entities::EntityColliderKind::Ball => ColliderBuilder::ball(half_height),
    };

    let entity_id: u128 = id.parse().expect("entity ID is not a number, impossible");
    builder
        .active_collision_types(ActiveCollisionTypes::all())
        .collision_groups(InteractionGroups::new(
            ENTITY_GROUP,
            BLOCK_GROUP | PLAYER_GROUP | ENTITY_GROUP,
        ))
        .user_data(entity_id)
        .position(vector![0., half_height, 0.].into())
        .build()
}

fn check_movement_for_collisions(
    player_collider_handle: ColliderHandle,
    desired_velocity: Vector3<f32>,
    physics_pipeline: &QueryPipeline,
    bodies: &RigidBodySet,
    colliders: &ColliderSet,
) -> CollisionResult {
    let mut corrected_velocity = desired_velocity;
    let mut remaining_time = TICK_DT;
    let mut is_on_ground = false;
    let mut would_have_collided = false;
    let player_collider = &colliders[player_collider_handle];
    let character_shape = player_collider.shape();
    let mut current_position = player_collider.position().translation.vector;

    const GROUND_THRESHOLD: f32 = 0.1; // Positive because ground normals are (0, 1, 0)
    const CEILING_THRESHOLD: f32 = -0.7; // Negative because ground normals are (0, 1, 0)

    let shape_cast_options = ShapeCastOptions {
        max_time_of_impact: 1.0,
        stop_at_penetration: false,
        ..Default::default()
    };

    for _ in 0..5 {
        let displacement = corrected_velocity * remaining_time;
        let shape_pos = current_position.into();

        if let Some((_handle, hit)) = physics_pipeline.cast_shape(
            bodies,
            colliders,
            &shape_pos,
            &displacement,
            character_shape,
            shape_cast_options,
            QueryFilter::default()
                .groups(InteractionGroups::new(
                    PLAYER_GROUP,
                    BLOCK_GROUP | PLAYER_GROUP,
                ))
                .exclude_collider(player_collider_handle),
        ) {
            would_have_collided = true;
            let toi = hit.time_of_impact;
            let normal = -hit.normal2.into_inner(); // Use normal2

            // Log the collision normal for debugging
            tracing::trace!("Collision normal: {:?}, status: {:?}", normal, hit.status);

            // Move up to the point of impact
            current_position += displacement * toi;

            // Sliding effect with separated adjustments
            let projection_len = corrected_velocity.dot(&normal);
            if projection_len < 0.0 {
                let adjustment = normal * projection_len;

                // if normal.y >= GROUND_THRESHOLD {
                //     // Collision with the ground
                //     corrected_velocity.y = 0.;
                // } else if normal.y <= CEILING_THRESHOLD {
                //     // Collision with the ceiling
                //     corrected_velocity.y -= adjustment.y;
                // } else {
                //     // Collision with walls
                //     corrected_velocity.x -= adjustment.x;
                //     corrected_velocity.z -= adjustment.z;
                // }

                // Note(ll): I'm trying this out because it fixes clipping
                // The idea to only correct the velocity if it is the opposite direction of the
                // normal is to always remove energy from the system, but not add any. This fixes
                // an issue where the player can also jump extra high by grazing a wall.
                if normal.y * corrected_velocity.y < 0. {
                    corrected_velocity.y -= adjustment.y;
                }
                if normal.x * corrected_velocity.x < 0. {
                    corrected_velocity.x -= adjustment.x;
                }
                if normal.z * corrected_velocity.z < 0. {
                    corrected_velocity.z -= adjustment.z;
                }
            }

            remaining_time *= 1.0 - toi;
        } else {
            current_position += displacement;
            break;
        }
    }

    // Double check that we're definitely not on the ground
    let ground_check_distance = 0.1;
    let down_direction = Vector3::new(0.0, -1.0, 0.0);
    let shape_cast_options = ShapeCastOptions {
        max_time_of_impact: 1.0,
        ..Default::default()
    };

    // No seriously, are we on the ground?
    if let Some((_handle, _hit)) = physics_pipeline.cast_shape(
        bodies,
        colliders,
        &current_position.into(),
        &(down_direction * ground_check_distance).into(),
        character_shape,
        shape_cast_options,
        QueryFilter::default().exclude_collider(player_collider_handle),
    ) {
        // OH BABY WE'RE ON THE GROUND
        is_on_ground = true;
        tracing::trace!("Aha!! We are on the ground");
    }

    CollisionResult {
        corrected_movement: na_to_glam(corrected_velocity),
        would_have_collided,
        is_on_ground,
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
    // Keep track of whether this handle has been removed from the physics world to warn the user
    // if a handle is dropped without being removed
    removed: bool,
}

impl PhysicsBody {
    pub fn new(handle: RigidBodyHandle) -> Self {
        Self {
            handle,
            removed: false,
        }
    }
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
}

impl<'a> DebugRenderBackend for PhysicsRenderer<'a> {
    fn draw_line(
        &mut self,
        _object: DebugRenderObject,
        a: Point<Real>,
        b: Point<Real>,
        _color: [f32; 4],
    ) {
        self.lines.push(net_types::DebugLine::new(
            na_point_to_glam(a),
            na_point_to_glam(b),
        ));
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CollisionResult {
    corrected_movement: glam::Vec3,
    would_have_collided: bool,
    is_on_ground: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Collision {
    collision_kind: CollisionKind,
    collision_target: CollisionTarget,
    target_id: String, // player ID if player, block type ID if block, entity ID if entity
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CollisionKind {
    Contact,
    Intersection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CollisionTarget {
    Block,
    Entity,
    Player,
}
