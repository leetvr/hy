#![cfg_attr(target_arch = "wasm32", no_main)]
#![cfg(not(target_arch = "wasm32"))]

mod game;
mod http;
mod js;

use {
    std::{str::FromStr, time::Instant},
    tracing_subscriber::{
        filter::{EnvFilter, LevelFilter},
        layer::SubscriberExt,
        util::SubscriberInitExt,
    },
};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // Start game server on a new thread
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        tracing::error!("Usage: {} <world directory>", args[0]);
        return;
    }
    let storage_dir = std::path::PathBuf::from_str(&args[1]).expect("Invalid storage dir path");

    if std::env::var("BROWSER") != Ok("none".to_owned()) {
        webbrowser::open("http://localhost:8888").expect("You.. don't have a web browser?");
    }

    tokio::join! {
        start_game_server(storage_dir),
        http::start_http_server()
    };
}

async fn start_game_server(storage_dir: std::path::PathBuf) {
    let mut last_tick = Instant::now();
    let mut server = game::GameServer::new(storage_dir).await;
    loop {
        server.tick().await;

        // sleep until the next tick
        let next_tick = last_tick + std::time::Duration::from_secs_f32(game::TICK_DT);
        last_tick = next_tick;
        tokio::time::sleep(next_tick - Instant::now()).await;
    }
}
