use deno_core::OpState;
use entities::{EntityData, EntityID, EntityState, EntityTypeID};
use net_types::{Controls, PlayerId};
use physics::PhysicsWorld;
use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
use {
    crate::game::PlayerCollision,
    deno_core::{
        extension, op2, serde_v8,
        v8::{self},
    },
};

use crate::game::{PlayerState, World};

#[op2]
#[serde]
// NOTE(kmrw: serde is apparently slow but who cares)
fn get_entities(state: &mut OpState) -> HashMap<EntityID, EntityData> {
    let world = state.borrow::<Arc<Mutex<World>>>();
    let world = world.lock().unwrap();

    world.entities.clone()
}

#[op2(fast)]
// NOTE(kmrw: serde is apparently slow but who cares)
fn is_player_on_ground(state: &mut OpState, #[bigint] player_id: u64) -> bool {
    let physics_world = state.borrow::<Arc<Mutex<PhysicsWorld>>>();
    let physics_world = physics_world.lock().expect("Deadlock!");

    physics_world.is_player_on_ground(player_id)
}

extension!(
    hy,
    ops = [get_entities, is_player_on_ground],
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

pub struct JSContext {
    runtime: deno_core::JsRuntime,
    // Safe to hold onto as long as the runtime is alive (probably)
    player_module_namespace: v8::Global<v8::Object>,
    entity_module_namespaces: Vec<v8::Global<v8::Object>>,
    world: Arc<Mutex<World>>,
}

impl JSContext {
    pub async fn new(
        script_root: impl Into<PathBuf>,
        world: Arc<Mutex<World>>,
        physics_world: Arc<Mutex<PhysicsWorld>>,
    ) -> anyhow::Result<Self> {
        // Get a clone the entity type registry before we pass it over to the runtime
        let entity_type_registry = {
            let world = world.lock().expect("Deadlock!");
            world.entity_type_registry.clone()
        };

        // Load the runtime
        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: vec![hy::init_ops_and_esm(world.clone(), physics_world)],
            ..Default::default()
        });

        let script_root: PathBuf = script_root.into();
        let player_script = script_root.join("player.js");

        // Load the player module
        let player_module_namespace = get_module_namespace(player_script, &mut runtime).await?;

        let mut entity_module_namespaces = Vec::new();

        // Load entity scripts
        // PARANOIA: Ensure we load the entity types in the correct order
        let mut entity_types = entity_type_registry.entity_types();
        entity_types.sort_by_key(|et| et.id());

        for entity_type in entity_type_registry.entity_types().iter() {
            let module_namespace =
                get_module_namespace(script_root.join(&entity_type.script_path()), &mut runtime)
                    .await?;
            entity_module_namespaces.push(module_namespace);
        }

        Ok(Self {
            runtime,
            player_module_namespace,
            entity_module_namespaces,
            world,
        })
    }

    pub async fn get_player_next_state(
        &mut self,
        player_id: PlayerId,
        current_state: &PlayerState,
        controls: &Controls,
        collisions: Vec<PlayerCollision>,
    ) -> anyhow::Result<PlayerState> {
        let scope = &mut self.runtime.handle_scope();
        let module_namespace = self.player_module_namespace.open(scope);

        let function_name = v8::String::new(scope, "update").unwrap();
        let Some(update_fn) = module_namespace.get(scope, function_name.into()) else {
            anyhow::bail!("ERROR: Module has no function named update!");
        };

        if !update_fn.is_function() {
            anyhow::bail!("ERROR: Module has a member named update, but it's not a function!");
        }

        let player_update = v8::Local::<v8::Function>::try_from(update_fn).unwrap(); // we know it's a function

        let undefined = deno_core::v8::undefined(scope).into();
        let player_id = serde_v8::to_v8(scope, player_id).unwrap();
        let current_state = serde_v8::to_v8(scope, current_state).unwrap();
        let controls = serde_v8::to_v8(scope, controls).unwrap();
        let colliding = serde_v8::to_v8(scope, collisions).unwrap();
        let args = [
            player_id.into(),
            current_state.into(),
            controls.into(),
            colliding.into(),
        ];

        let result = player_update.call(scope, undefined, &args).unwrap();
        let next_state: PlayerState = serde_v8::from_v8(scope, result)?;

        Ok(next_state)
    }

    pub async fn run_script_for_entity(
        &mut self,
        entity_id: &str,
        entity_type_id: EntityTypeID,
    ) -> anyhow::Result<()> {
        let scope = &mut self.runtime.handle_scope();
        let module_namespace = &self.entity_module_namespaces[entity_type_id as usize];
        let module_namespace = module_namespace.open(scope);

        let function_name = v8::String::new(scope, "update").unwrap();
        let Some(update_fn) = module_namespace.get(scope, function_name.into()) else {
            anyhow::bail!("ERROR: Module has no function named update!");
        };

        if !update_fn.is_function() {
            anyhow::bail!("ERROR: Module has a member named update, but it's not a function!");
        }

        let entity_update = v8::Local::<v8::Function>::try_from(update_fn).unwrap(); // we know it's a function

        let undefined = deno_core::v8::undefined(scope).into();
        let current_state = {
            let world = self.world.lock().expect("Deadlock!");
            let Some(entity_data) = &world.entities.get(entity_id) else {
                tracing::error!("Attempted to update entity that does not exist: {entity_id}.");
                return Ok(());
            };
            serde_v8::to_v8(scope, &entity_data.state).unwrap()
        };

        let args = [current_state.into()];

        let result = entity_update.call(scope, undefined, &args).unwrap();

        // Get the entity's next state
        let next_state: EntityState = serde_v8::from_v8(scope, result)?;

        // Update the entity
        {
            let mut world = self.world.lock().expect("Deadlock!");
            world.entities.get_mut(entity_id).unwrap().state = next_state;
        }

        Ok(())
    }
}

async fn get_module_namespace(
    script_path: PathBuf,
    runtime: &mut deno_core::JsRuntime,
) -> Result<v8::Global<v8::Object>, anyhow::Error> {
    tracing::debug!("Loading script at {script_path:?}");
    let module = deno_core::resolve_path(script_path, &std::env::current_dir()?)?;
    let module_id = runtime.load_side_es_module(&module).await?;
    runtime.mod_evaluate(module_id).await?;
    runtime.run_event_loop(Default::default()).await?;
    let module_namespace = runtime.get_module_namespace(module_id)?;
    Ok(module_namespace)
}
