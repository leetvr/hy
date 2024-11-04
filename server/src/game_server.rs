use {
    anyhow::Result,
    futures_util::{SinkExt, StreamExt},
    tokio::net::TcpListener,
    tokio_tungstenite::tungstenite::Message,
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

            let mut position = glam::Vec2::ZERO;

            let (mut write, mut read) = ws_stream.split();
            loop {
                if let Some(message) = read.next().await {
                    let controls = match message {
                        Ok(v) => match bincode::deserialize::<net::Controls>(&v.into_data()) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::info!("Error deserializing controls: {}", e);
                                break;
                            }
                        },
                        Err(e) => {
                            tracing::info!("Error receiving message: {}", e);
                            break;
                        }
                    };

                    position += controls.move_direction * PLAYER_SPEED;

                    let position_message = bincode::serialize(&net::Position(position)).unwrap();
                    if let Err(e) = write.send(Message::Binary(position_message)).await {
                        tracing::info!("Error sending message: {}", e);
                        break;
                    }
                }
            }
        });
    }

    Ok(())
}

const PLAYER_SPEED: f32 = 0.05;
