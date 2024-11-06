use deno_core::{error::AnyError, extension, op2, v8};
use std::{net::SocketAddr, rc::Rc};

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
    esm = [dir "src", "runtime.js"]
);

pub async fn run_js(file_path: &str, addr: SocketAddr) -> anyhow::Result<String> {
    let file_path = &file_path[1..];
    tracing::info!("CLIENT <- POST {file_path}");
    let Ok(main_module) = deno_core::resolve_path(file_path, &std::env::current_dir()?) else {
        return Ok(format!("ERROR: File not found: {file_path}"));
    };

    // Load the runtime
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![crimes::init_ops_and_esm()],
        ..Default::default()
    });

    let Ok(mod_id) = js_runtime.load_main_es_module(&main_module).await else {
        return Ok(format!("ERROR: File not found: {file_path}"));
    };

    js_runtime.mod_evaluate(mod_id).await?;
    js_runtime.run_event_loop(Default::default()).await?;

    let module_namespace = js_runtime.get_module_namespace(mod_id)?;
    let promise = {
        let scope = &mut js_runtime.handle_scope();
        let module_namespace = module_namespace.open(scope);

        let function_name = v8::String::new(scope, "greet").unwrap();
        let Some(greet) = module_namespace.get(scope, function_name.into()) else {
            return Ok(format!("ERROR: Module has no function named greet!"));
        };

        if !greet.is_function() {
            return Ok(format!("ERROR: Module has no function named greet!"));
        }

        let greet = v8::Local::<v8::Function>::try_from(greet).unwrap(); // we know it's a function

        let undefined = deno_core::v8::undefined(scope).into();
        let arg = v8::String::new(scope, &addr.to_string()).unwrap();
        let args = [arg.into()];

        let promise = greet.call(scope, undefined, &args).unwrap();

        if !promise.is_promise() {
            return Ok(format!("ERROR: greet did not return a promise!"));
        }

        v8::Global::new(scope, promise)
    };

    let result = {
        let value = js_runtime.resolve(promise);
        js_runtime.run_event_loop(Default::default()).await?;
        let scope = &mut js_runtime.handle_scope();

        value
            .await?
            .open(scope)
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    };

    Ok(result)
}
