#![cfg_attr(target_arch = "wasm32", no_main)]
#![cfg(not(target_arch = "wasm32"))]

mod game_server;
mod js;

use {
    anyhow::Result,
    futures_util::try_join,
    js::run_js,
    std::net::SocketAddr,
    tokio::{
        fs,
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    },
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

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    webbrowser::open("http://localhost:8888").expect("You.. don't have a web browser?");

    let game = async {
        tokio::spawn(game_server::start_game_server())
            .await
            .unwrap()
    };
    let http = start_http_server();

    runtime.block_on(async { try_join!(game, http) }).unwrap();
}

async fn start_http_server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8888").await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        handle_connection(stream, addr).await?;
    }
}

async fn handle_connection(mut stream: TcpStream, addr: SocketAddr) -> Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await?;

    let request = String::from_utf8_lossy(&buffer[..]);
    let mut lines = request.lines();
    let first_line = lines.next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let response = match method {
        "GET" => handle_get(path).await?,
        "POST" => handle_post(path, addr).await?,
        _ => "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n"
            .as_bytes()
            .to_vec(),
    };

    stream.write_all(&response).await?;
    stream.flush().await?;

    Ok(())
}

async fn handle_post(path: &str, addr: SocketAddr) -> Result<Vec<u8>> {
    let response = run_js(path, addr).await?;
    Ok(format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{response}",
        response.len()
    )
    .into())
}

async fn handle_get(request_path: &str) -> Result<Vec<u8>> {
    tracing::info!("CLIENT <- GET {request_path}");
    let request_path = if request_path == "/" {
        "/index.html"
    } else {
        request_path
    };

    let mut file_path = std::path::PathBuf::from("./client/");
    file_path.push(&request_path[1..]); // Remove leading '/'

    if !file_path.exists() {
        tracing::info!("SERVER <- 404");
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes().to_vec();
        return Ok(response);
    }

    let contents = fs::read(&file_path).await?;
    let content_type = if request_path.ends_with(".wasm") {
        "application/wasm"
    } else if request_path.ends_with(".js") {
        "application/javascript"
    } else if request_path.ends_with(".html") {
        "text/html"
    } else {
        "application/octet-stream"
    };

    tracing::info!("SERVER -> 200 {file_path:?}");

    Ok(format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n",
        contents.len(),
        content_type
    )
    .into_bytes()
    .into_iter()
    .chain(contents.into_iter())
    .collect())
}
