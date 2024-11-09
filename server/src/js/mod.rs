use deno_core::{
    error::AnyError,
    extension, op2, serde_v8,
    v8::{self},
};
use net_types::Controls;
use std::{path::Path, rc::Rc};

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
}

impl JSContext {
    pub async fn new(player_module_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        // Load the runtime
        let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: vec![crimes::init_ops_and_esm()],
            ..Default::default()
        });

        // Load the player module
        let player_module = deno_core::resolve_path(player_module_path, &std::env::current_dir()?)?;
        let player_module_id = runtime.load_main_es_module(&player_module).await?;
        runtime.mod_evaluate(player_module_id).await?;
        runtime.run_event_loop(Default::default()).await?;
        let player_module_namespace = runtime.get_module_namespace(player_module_id)?;

        Ok(Self {
            runtime,
            player_module_namespace,
        })
    }

    pub async fn get_player_next_position(
        &mut self,
        current_position: &glam::Vec3,
        controls: &Controls,
    ) -> anyhow::Result<glam::Vec3> {
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
        let current_position = serde_v8::to_v8(scope, current_position).unwrap();
        let controls = serde_v8::to_v8(scope, controls).unwrap();
        let args = [current_position.into(), controls.into()];

        let result = player_update.call(scope, undefined, &args).unwrap();
        let next_position: glam::Vec3 = serde_v8::from_v8(scope, result)?;

        Ok(next_position)
    }
}
