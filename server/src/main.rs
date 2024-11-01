use anyhow::Result;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8888").await?;
    webbrowser::open("http://localhost:8888").expect("You.. don't have a web browser?");

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await?;

    let request = String::from_utf8_lossy(&buffer[..]);
    let mut lines = request.lines();
    let first_line = lines.next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let response = get_response(path).await?;

    stream.write_all(&response).await?;
    stream.flush().await?;

    Ok(())
}

async fn get_response(request_path: &str) -> Result<Vec<u8>> {
    println!("CLIENT <- {request_path}");
    let request_path = if request_path == "/" {
        "/index.html"
    } else {
        request_path
    };

    let mut file_path = std::path::PathBuf::from("./client/");
    file_path.push(&request_path[1..]); // Remove leading '/'

    if !file_path.exists() {
        println!("SERVER <- 404");
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

    println!("SERVER -> 200 {file_path:?}");

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
