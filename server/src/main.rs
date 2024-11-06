#![cfg_attr(target_arch = "wasm32", no_main)]
#![cfg(not(target_arch = "wasm32"))]

mod game;
mod http;
mod js;

use {
    std::time::Instant,
    tracing_subscriber::{
        filter::{EnvFilter, LevelFilter},
        layer::SubscriberExt,
        util::SubscriberInitExt,
    },
};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Start game server on a new thread
    let spawner = runtime.handle().clone();
    std::thread::spawn(move || {
        let mut server = game::GameServer::new(spawner);

        let mut last_tick = Instant::now();
        loop {
            server.tick();

            // sleep until the next tick
            let next_tick = last_tick + std::time::Duration::from_secs_f32(game::TICK_DT);
            last_tick = next_tick;
            std::thread::sleep(next_tick - Instant::now());
        }
    });

    webbrowser::open("http://localhost:8888").expect("You.. don't have a web browser?");

    // start_http_server is !Send, so we need to await it on the current thread
    let _ = runtime.block_on(http::start_http_server());
}
