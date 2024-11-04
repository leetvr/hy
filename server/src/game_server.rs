use {
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    tokio::net::TcpListener,
};

pub async fn start_game_server() -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:8889").await?;

    while let Ok((stream, _)) = server.accept().await {
        // Echo server
        tokio::spawn(async move {
            let addr = stream.peer_addr().expect("no peer address found");
            tracing::info!("Client connected from: {}", addr);

            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("error during the websocket handshake");

            let (mut write, mut read) = ws_stream.split();
            loop {
                if let Some(message) = read.next().await {
                    let message = match message {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::info!("Error receiving message: {}", e);
                            break;
                        }
                    };

                    match message.into_text() {
                        Ok(v) => {
                            tracing::info!("Received message: {}", v);
                            write.send(v.into()).await.expect("error sending message");
                        }
                        Err(e) => {
                            tracing::info!("Error decoding message into string: {}", e);
                            break;
                        }
                    }
                }
            }
        });
    }

    Ok(())
}
