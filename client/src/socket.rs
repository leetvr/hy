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

    // Read incoming messages to a queue
    let onmessage = Closure::wrap(Box::new({
        let incoming_messages = incoming_messages.clone();
        move |event: MessageEvent| {
            let message = event
                .data()
                .dyn_into::<Blob>()
                .expect("Failed to read message");

            // Note(ll): I think this can be done synchronously
            let incoming_messages = incoming_messages.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let array_buffer = JsFuture::from(message.array_buffer())
                    .await
                    .expect("Failed to read array buffer")
                    .dyn_into::<ArrayBuffer>()
                    .unwrap();
                let array = Uint8Array::new(&array_buffer);
                incoming_messages.borrow_mut().push(array.to_vec());
            });
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

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
