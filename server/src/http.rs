use std::net::SocketAddr;

use anyhow::Result;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::js::run_js;

/// If you're going to make your protocol string based, then I'm going to implement it with string
/// manipulation.
pub async fn start_http_server() -> Result<()> {
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
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{response}\r\n",
        response.len() + 2 // add 2 bytes for the CRLF
    )
    .into())
}

async fn handle_get(request_path: &str) -> Result<Vec<u8>> {
    tracing::info!("CLIENT <- GET {request_path}");
    let request_path = if request_path == "/" {
        "assets/index.html"
    } else {
        &request_path[1..] // remove leading '/'
    };

    let file_path = std::path::Path::new(request_path);

    if !file_path.exists() {
        tracing::error!("{file_path:?} does not exist");
        tracing::info!("SERVER -> 404");
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes().to_vec();
        return Ok(response);
    }

    let contents = fs::read(&file_path).await?;
    let content_type = get_mime_type(request_path);

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

fn get_mime_type(request_path: &str) -> &str {
    if request_path.ends_with(".wasm") {
        "application/wasm"
    } else if request_path.ends_with(".js") {
        "application/javascript"
    } else if request_path.ends_with(".html") {
        "text/html"
    } else if request_path.ends_with(".css") {
        "text/css"
    } else {
        "application/octet-stream"
    }
}
