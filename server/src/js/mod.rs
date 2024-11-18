mod extensions;

use entities::{EntityData, EntityTypeID};
use extensions::hy;
use physics::PhysicsWorld;
use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
use {
    crate::game::{PlayerState, World},
    deno_core::{
        op2, serde_v8,
        v8::{self},
        OpState,
    },
    entities::{EntityID, PlayerId},
    std::collections::HashMap,
};
use {entities::EntityState, net_types::Controls};

#[op2]
#[serde]
// NOTE(kmrw: serde is apparently slow but who cares)
fn get_entities(state: &mut OpState) -> HashMap<EntityID, EntityData> {
    // tracing::info!("Get entities called");
    let shared_state = state.borrow::<Arc<Mutex<World>>>();
    let state = shared_state.lock().unwrap();

    state.entities.clone()
}

pub struct JSContext {
    runtime: deno_core::JsRuntime,
    // Safe to hold onto as long as the runtime is alive (probably)
    world_module_namespace: v8::Global<v8::Object>,
    player_module_namespace: v8::Global<v8::Object>,
    entity_module_namespaces: HashMap<String, v8::Global<v8::Object>>, // indexed by path
    entity_module_paths: Vec<String>,                                  // indexed by entity type ID
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

        // Load the world module
        let world_module_namespace =
            get_module_namespace(script_root.join("world.js"), &mut runtime).await?;

        // Load the player module
        let player_script = script_root.join("player.js");
        let player_module_namespace = get_module_namespace(player_script, &mut runtime).await?;

        let mut entity_module_namespaces = HashMap::new();
        let mut entity_module_paths = Vec::new();

        // Load entity scripts
        // PARANOIA: Ensure we load the entity types in the correct order
        let mut entity_types = entity_type_registry.entity_types();
        entity_types.sort_by_key(|et| et.id());

        for entity_type in entity_type_registry.entity_types().iter() {
            let path = entity_type.script_path();

            // IMPORTANT: Deno will get very mad if we load the same module twice.
            if !entity_module_namespaces.contains_key(path) {
                let module_namespace =
                    get_module_namespace(script_root.join(path), &mut runtime).await?;
                entity_module_namespaces.insert(path.to_string(), module_namespace);
            }

            entity_module_paths.push(path.to_string());
        }

        Ok(Self {
            runtime,
            world_module_namespace,
            player_module_namespace,
            entity_module_namespaces,
            entity_module_paths,
            world,
        })
    }

    pub async fn get_player_next_state(
        &mut self,
        player_id: PlayerId,
        current_state: &PlayerState,
        controls: &Controls,
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
        let args = [player_id.into(), current_state.into(), controls.into()];

        let result = player_update.call(scope, undefined, &args).unwrap();
        let next_state: PlayerState = serde_v8::from_v8(scope, result)?;

        Ok(next_state)
    }

    pub(crate) fn get_player_spawn_state(
        &mut self,
        player_id: PlayerId,
        current_state: &PlayerState,
    ) -> anyhow::Result<PlayerState> {
        // This is the same as get_player_next_state, but with a different function name, and no
        // controls parameter
        // Laziness made this happen
        let scope = &mut self.runtime.handle_scope();
        let module_namespace = self.player_module_namespace.open(scope);

        let function_name = v8::String::new(scope, "onSpawn").unwrap();
        let Some(update_fn) = module_namespace.get(scope, function_name.into()) else {
            anyhow::bail!("ERROR: Module has no function named onSpawn!");
        };

        if !update_fn.is_function() {
            anyhow::bail!("ERROR: Module has a member named onSpawn, but it's not a function!");
        }

        let player_update = v8::Local::<v8::Function>::try_from(update_fn).unwrap(); // we know it's a function

        let undefined = deno_core::v8::undefined(scope).into();
        let player_id = serde_v8::to_v8(scope, player_id).unwrap();
        let current_state = serde_v8::to_v8(scope, current_state).unwrap();
        let args = [player_id.into(), current_state.into()];

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
        let module_path = &self.entity_module_paths[entity_type_id as usize];
        let module_namespace = &self.entity_module_namespaces[module_path];
        let module_namespace = module_namespace.open(scope);

        // Get the update function
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
            let mut world = self.world.lock().expect("Deadlock!");
            let Some(entity_data) = world.entities.get_mut(entity_id) else {
                tracing::error!("Attempted to update entity that does not exist: {entity_id}.");
                return Ok(());
            };

            let state = serde_v8::to_v8(scope, &entity_data.state).unwrap();
            entity_data.state.interactions.clear();

            state
        };

        let entity_id_arg = serde_v8::to_v8(scope, entity_id).unwrap();
        let args = [entity_id_arg.into(), current_state.into()];

        // Call the function
        let result = entity_update.call(scope, undefined, &args).unwrap();

        // Get the entity's next state
        let next_state = serde_v8::from_v8(scope, result)?;

        // Update the entity
        {
            let mut world = self.world.lock().expect("Deadlock!");
            let entity = world.entities.get_mut(entity_id).unwrap();

            let EntityState {
                position,
                rotation,
                velocity,
                custom_state,

                // Not mutable from JS
                anchor: _,
                interactions: _,
            } = next_state;

            entity.state.position = position;
            entity.state.rotation = rotation;
            entity.state.velocity = velocity;
            entity.state.custom_state = custom_state;
        }

        Ok(())
    }

    pub(crate) fn spawn_entity(&mut self, entity_data: &mut EntityData) {
        // Load the module for this entity type
        let scope = &mut self.runtime.handle_scope();
        let module_path = &self.entity_module_paths[entity_data.entity_type as usize];
        let module_namespace = &self.entity_module_namespaces[module_path];
        let module_namespace = module_namespace.open(scope);

        // Check whether this entity type has an onSpawn function
        let function_name = v8::String::new(scope, "onSpawn").unwrap();
        let Some(on_spawn) = module_namespace.get(scope, function_name.into()) else {
            return;
        };

        // Since onSpawn is optional, if there's no function then just return
        if !on_spawn.is_function() {
            return;
        }

        // Get the actual function
        let on_spawn = v8::Local::<v8::Function>::try_from(on_spawn).unwrap(); // we know it's a function

        // Put the args together
        let undefined = deno_core::v8::undefined(scope).into();
        let initial_state = serde_v8::to_v8(scope, &entity_data).unwrap();

        let args = [initial_state.into()];

        // Call the function
        let result = on_spawn.call(scope, undefined, &args).unwrap();

        // Get the entity's initial state
        let EntityData {
            name,
            model_path,
            state,
            ..
        } = serde_v8::from_v8(scope, result).unwrap();

        // Update the entity
        entity_data.name = name;
        entity_data.model_path = model_path;
        entity_data.state = state;
    }

    pub(crate) fn run_world_spawn_player_script(
        &mut self,
        custom_world_state: &mut serde_json::Value,
        player_id: PlayerId,
        current_state: &PlayerState,
    ) -> anyhow::Result<PlayerState> {
        let scope = &mut self.runtime.handle_scope();
        let module_namespace = self.world_module_namespace.open(scope);

        let function_name = v8::String::new(scope, "onAddPlayer").unwrap();
        let Some(update_fn) = module_namespace.get(scope, function_name.into()) else {
            anyhow::bail!("ERROR: Module has no function named onAddPlayer!");
        };

        if !update_fn.is_function() {
            anyhow::bail!("ERROR: Module has a member named onAddPlayer, but it's not a function!");
        }

        let player_update = v8::Local::<v8::Function>::try_from(update_fn).unwrap(); // we know it's a function

        let undefined = deno_core::v8::undefined(scope).into();
        let world_state = serde_v8::to_v8(scope, &*custom_world_state).unwrap();
        let player_id = serde_v8::to_v8(scope, player_id).unwrap();
        let current_state = serde_v8::to_v8(scope, current_state).unwrap();
        let args = [world_state.into(), player_id.into(), current_state.into()];

        let result = player_update.call(scope, undefined, &args).unwrap();
        let (next_world_state, next_player_state): (serde_json::Value, PlayerState) =
            serde_v8::from_v8(scope, result)?;

        *custom_world_state = next_world_state;

        Ok(next_player_state)
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
