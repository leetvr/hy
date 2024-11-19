use {
    crate::game::{PlayerState, World},
    anyhow::bail,
    deno_core::{error::AnyError, extension, op2, OpState},
    entities::{EntityData, EntityID, EntityState, PlayerId},
    glam::{EulerRot, Vec3},
    nanorand::Rng,
    physics::{Collision, CollisionResult, PhysicsWorld},
    std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    },
};

#[op2]
#[serde]
fn get_player_state(state: &mut OpState, #[bigint] player_id: u64) -> Option<PlayerState> {
    let world = state.borrow::<Arc<Mutex<World>>>();
    let world = world.lock().unwrap();

    world.player_data.get(&PlayerId::new(player_id)).cloned()
}

#[op2]
#[serde]
// NOTE(kmrw: serde is apparently slow but who cares)
fn get_entities(state: &mut OpState) -> HashMap<EntityID, EntityData> {
    let world = state.borrow::<Arc<Mutex<World>>>();
    let world = world.lock().unwrap();

    world.entities.clone()
}

#[op2]
#[serde]
fn get_entity_data(state: &mut OpState, #[string] entity_id: String) -> Option<EntityData> {
    let world = state.borrow::<Arc<Mutex<World>>>();
    let world = world.lock().unwrap();

    world.entities.get(&entity_id).cloned()
}

#[op2]
#[serde]
// NOTE(kmrw: serde is apparently slow but who cares)
fn check_movement_for_collisions(
    state: &mut OpState,
    #[bigint] player_id: u64,
    #[serde] current_position: glam::Vec3,
    #[serde] movement: glam::Vec3,
) -> CollisionResult {
    let physics_world = state.borrow::<Arc<Mutex<PhysicsWorld>>>();
    let mut physics_world = physics_world.lock().expect("Deadlock!");

    physics_world.check_movement_for_collisions(player_id, current_position, movement)
}

#[op2]
#[serde]
fn get_collisions_for_entity(state: &mut OpState, #[string] entity_id: String) -> Vec<Collision> {
    let physics_world = state.borrow::<Arc<Mutex<PhysicsWorld>>>();
    let physics_world = physics_world.lock().expect("Deadlock!");

    physics_world.get_collisions_for_entity(&entity_id)
}

#[op2]
#[serde]
fn get_collisions_for_player(state: &mut OpState, #[bigint] player_id: u64) -> Vec<Collision> {
    let physics_world = state.borrow::<Arc<Mutex<PhysicsWorld>>>();
    let physics_world = physics_world.lock().expect("Deadlock!");

    physics_world.get_collisions_for_player(player_id)
}

#[op2]
#[serde]
fn spawn_entity(
    state: &mut OpState,
    entity_type_id: u8,
    #[serde] position: glam::Vec3,
    #[serde] rotation: glam::Vec3,
    #[serde] velocity: glam::Vec3,
    #[serde] custom_state: Option<HashMap<String, serde_json::Value>>,
) -> Result<EntityID, AnyError> {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();

    let Some(entity_type) = world.entity_type_registry.get(entity_type_id) else {
        bail!("Entity type not found");
    };

    let entity_id = nanorand::tls_rng().generate::<u64>().to_string();
    let entity_data = EntityData {
        id: entity_id.clone(),
        name: "We should let you set entity names in the editor".into(),
        entity_type: entity_type.id,
        model_path: entity_type.default_model_path().into(),
        state: EntityState {
            position: position.into(),
            rotation: glam::Quat::from_euler(EulerRot::YXZ, rotation.y, rotation.x, rotation.z),
            velocity: velocity.into(),
            custom_state: custom_state.unwrap_or_default(),
            ..Default::default()
        },
    };

    world.spawn_entity(entity_id.clone(), entity_data);

    Ok(entity_id)
}

#[op2(fast)]
fn despawn_entity(state: &mut OpState, #[string] entity_id: String) {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();

    world.despawn_entity(entity_id);
}

#[op2(fast)]
fn anchor_entity(
    state: &mut OpState,
    #[string] entity_id: String,
    #[bigint] player_id: u64,
    #[string] anchor_name: String,
) {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();

    world.anchor_entity(entity_id, player_id, anchor_name);
}

#[op2]
fn detach_entity(state: &mut OpState, #[string] entity_id: String, #[serde] position: Vec3) {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();

    world.detach_entity(entity_id, position);
}

#[op2]
fn interact_entity(
    state: &mut OpState,
    #[string] entity_id: String,
    #[bigint] player_id: u64,
    #[serde] position: Vec3,
    facing_angle: f32,
    #[serde] custom_state: Option<HashMap<String, serde_json::Value>>,
) {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();

    world.interact_entity(
        entity_id,
        PlayerId::new(player_id),
        position,
        facing_angle,
        custom_state.unwrap_or_default(),
    );
}

#[op2]
#[serde]
fn play_sound(
    state: &mut OpState,
    #[string] sound_id: String,
    #[serde] position: glam::Vec3,
    volume: f32,
) {
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let mut world = shared_state.lock().unwrap();
    world.play_sound(sound_id, position, volume)
}

// Exports the extensions as a variable named `hy`
extension!(
    hy,
    ops = [
        get_player_state,
        get_entities,
        get_entity_data,
        check_movement_for_collisions,
        spawn_entity,
        despawn_entity,
        anchor_entity,
        detach_entity,
        interact_entity,
        get_collisions_for_entity,
        get_collisions_for_player,
        play_sound,
    ],
    esm_entry_point = "ext:hy/runtime.js",
    esm = [dir "src/js", "runtime.js"],
    options = {
        world: Arc<Mutex<World>>,
        physics_world: Arc<Mutex<PhysicsWorld>>,
    },
    state = |state, options| {
        state.put(options.world.clone());
        state.put(options.physics_world.clone());
    }
);
