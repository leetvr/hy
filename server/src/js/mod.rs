use entities::{EntityData, EntityState, EntityTypeRegistry};
use net_types::Controls;
use std::{path::PathBuf, rc::Rc};
use {
    crate::game::PlayerCollision,
    deno_core::{
        error::AnyError,
        extension, op2, serde_v8,
        v8::{self},
    },
};

use crate::game::PlayerState;

#[op2(async)]
#[string]
async fn hello(#[string] ip: String) -> Result<String, AnyError> {
    tracing::info!("Hello from Rust! I was called with {ip}!");
    Ok(format!("Your IP is {ip}, but from Rust"))
}

extension!(
    crimes,
    ops = [hello],
    esm_entry_point = "ext:crimes/runtime.js",
    esm = [dir "src/js", "runtime.js"]
);

pub struct JSContext {
    runtime: deno_core::JsRuntime,
    // Safe to hold onto as long as the runtime is alive (probably)
    player_module_namespace: v8::Global<v8::Object>,
    entity_module_namespaces: Vec<v8::Global<v8::Object>>,
}

impl JSContext {
    pub async fn new(
        script_root: impl Into<PathBuf>,
        entity_type_registry: &EntityTypeRegistry,
    ) -> anyhow::Result<Self> {
        // Load the runtime
        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: vec![crimes::init_ops_and_esm()],
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
        entity_types.sort_by_key(|et| et.id);

        for entity_type in entity_type_registry.entity_types().iter() {
            let module_namespace =
                get_module_namespace(script_root.join(&entity_type.script_path), &mut runtime)
                    .await?;
            entity_module_namespaces.push(module_namespace);
        }

        Ok(Self {
            runtime,
            player_module_namespace,
            entity_module_namespaces,
        })
    }

    pub async fn get_player_next_state(
        &mut self,
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
        let current_state = serde_v8::to_v8(scope, current_state).unwrap();
        let controls = serde_v8::to_v8(scope, controls).unwrap();
        let colliding = serde_v8::to_v8(scope, collisions).unwrap();
        let args = [current_state.into(), controls.into(), colliding.into()];

        let result = player_update.call(scope, undefined, &args).unwrap();
        let next_state: PlayerState = serde_v8::from_v8(scope, result)?;

        Ok(next_state)
    }

    pub async fn get_entity_next_state(
        &mut self,
        entity: &EntityData,
    ) -> anyhow::Result<EntityState> {
        let scope = &mut self.runtime.handle_scope();
        let module_namespace = &self.entity_module_namespaces[entity.entity_type as usize];
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
        let current_state = serde_v8::to_v8(scope, &entity.state).unwrap();
        let args = [current_state.into()];

        let result = entity_update.call(scope, undefined, &args).unwrap();
        let next_state: EntityState = serde_v8::from_v8(scope, result)?;

        Ok(next_state)
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
