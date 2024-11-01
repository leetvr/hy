use deno_core::{error::AnyError, extension, op2, v8::Value};
use std::{borrow::Borrow, cell::RefCell, collections::HashMap, net::SocketAddr, rc::Rc};

thread_local! {
    static KEYDOWN_HANDLER: RefCell<HashMap<SocketAddr, String>> = RefCell::new(HashMap::new());
}

#[op2(async)]
#[string]
async fn hello(#[string] ip: String) -> Result<String, AnyError> {
    println!("Hello called: {ip}!");
    Ok(format!("Hello {ip} from Rust"))
}

extension!(
    crimes,
    ops = [hello],
    esm_entry_point = "ext:crimes/runtime.js",
    esm = [dir "src", "runtime.js"]
);

pub async fn run_js(file_path: &str, addr: SocketAddr) -> anyhow::Result<String> {
    let file_path = &file_path[1..];
    println!("CLIENT <- POST {file_path}");
    let Ok(main_module) = deno_core::resolve_path(file_path, &std::env::current_dir()?) else {
        return Ok(format!("ERROR: File not found: {file_path}"));
    };

    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![crimes::init_ops_and_esm()],
        ..Default::default()
    });

    // Load the runtime
    let Ok(mod_id) = js_runtime.load_main_es_module(&main_module).await else {
        return Ok(format!("ERROR: File not found: {file_path}"));
    };

    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(Default::default()).await?;

    result.await?;

    let SocketAddr::V4(addr) = addr else {
        return Ok("Wow, you're using IPv6!".into());
    };

    let ip = addr.ip().to_string();

    let script = format!("crimes.hello(\"{ip}\").then(console.log);");

    println!("Evaluating {script}");
    let promise = js_runtime.execute_script("", script)?;
    {
        let promise: &Value = &promise.borrow();
        println!("is_promise {}", promise.is_promise());
    }

    // This line seems to hang the runtime
    let resolved = js_runtime.resolve(promise).await?;

    let value: &Value = resolved.borrow();
    let value = value.to_rust_string_lossy(&mut js_runtime.handle_scope());

    Ok(value)
}
