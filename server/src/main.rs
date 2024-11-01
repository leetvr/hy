use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8888").unwrap();
    println!("Serving on http://127.0.0.1:8888");

    webbrowser::open("http://localhost:8888").expect("You.. don't have a web browser?");

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        thread::spawn(move || {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: std::net::TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);
    let mut lines = request.lines();
    let first_line = lines.next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let response = get_response(path);

    stream.write_all(&response).unwrap();
    stream.flush().unwrap();
}

fn get_response(request_path: &str) -> Vec<u8> {
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
        return "HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes().to_vec();
    }

    let contents = fs::read(&file_path).unwrap();
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

    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n",
        contents.len(),
        content_type
    )
    .into_bytes()
    .into_iter()
    .chain(contents.into_iter())
    .collect()
}
