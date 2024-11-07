use {
    anyhow::Result,
    std::{cell::RefCell, mem, rc::Rc},
    wasm_bindgen::{prelude::Closure, JsCast},
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        js_sys::{ArrayBuffer, Uint8Array},
        Blob, Event, MessageEvent, WebSocket,
    },
};

pub fn connect_to_server(
    addr: &str,
    connection_state: Rc<RefCell<ConnectionState>>,
    incoming_messages: Rc<RefCell<Vec<Vec<u8>>>>,
) -> Result<WebSocket> {
    // Connect to server
    let ws = WebSocket::new(addr).expect("Failed to connect to server");

    let (blob_tx, mut blob_rx) = tokio::sync::mpsc::channel::<Blob>(16);
    // Read incoming messages to a queue
    let onmessage = Closure::wrap(Box::new({
        move |event: MessageEvent| {
            let message = event
                .data()
                .dyn_into::<Blob>()
                .expect("Failed to read message");

            let _ = blob_tx.blocking_send(message);
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    // Spawn a task for reading incoming blobs in the order they are received
    // This is necessary because `blob.array_buffer()` is asynchronous
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(blob) = blob_rx.recv().await {
            let array_buffer = JsFuture::from(blob.array_buffer())
                .await
                .expect("Failed to read array buffer")
                .dyn_into::<ArrayBuffer>()
                .unwrap();
            let array = Uint8Array::new(&array_buffer);
            let data = array.to_vec();
            incoming_messages.borrow_mut().push(data);
        }
    });

    // Handle changes in connection states
    let onopen = Closure::wrap(Box::new({
        let connection_state = connection_state.clone();
        move || {
            *connection_state.borrow_mut() = ConnectionState::Connected;
        }
    }) as Box<dyn FnMut()>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    let onclose = Closure::wrap(Box::new({
        let connection_state = connection_state.clone();
        move || {
            *connection_state.borrow_mut() = ConnectionState::Closed;
        }
    }) as Box<dyn FnMut()>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    let onerror = Closure::wrap(Box::new({
        let connection_state = connection_state.clone();
        move |event: Event| {
            *connection_state.borrow_mut() = ConnectionState::Error(event.as_string().unwrap());
        }
    }) as Box<dyn FnMut(Event)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    // Callback closures need to stay alive. There are two ways of doing this:
    // 1. Hold on to the closure reference so it's never dropped
    // 2. Leak the closure reference so it's never dropped
    //
    // I'm doing the bad one
    mem::forget(onmessage);
    mem::forget(onopen);
    mem::forget(onclose);
    mem::forget(onerror);

    Ok(ws)
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Closed,
    Error(String),
}
