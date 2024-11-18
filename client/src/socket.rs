use {
    anyhow::Result,
    net_types::ServerPacket,
    std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
        mem,
        rc::Rc,
    },
    wasm_bindgen::{prelude::Closure, JsCast},
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        js_sys::{ArrayBuffer, Uint8Array},
        Blob, Event, MessageEvent, WebSocket,
    },
};

thread_local! {
    static SEQUENCE_COUNTER: Cell<u64> = Cell::new(0);
}

pub type IncomingMessages = Rc<RefCell<BTreeMap<u64, ServerPacket>>>;

pub fn connect_to_server(
    addr: &str,
    connection_state: Rc<RefCell<ConnectionState>>,
    incoming_messages: IncomingMessages,
) -> Result<WebSocket> {
    // Connect to server
    let ws = WebSocket::new(addr).expect("Failed to connect to server");

    // Read incoming messages to a queue
    let onmessage = Closure::wrap(Box::new({
        move |event: MessageEvent| {
            let blob = event
                .data()
                .dyn_into::<Blob>()
                .expect("Failed to read message");

            // We need to to this because reasons. Ask Kane or Lilith, but they've probably
            // forgotten already.
            let incoming_messages = incoming_messages.clone();
            let sequence_number = SEQUENCE_COUNTER.with(|counter| {
                let current = counter.get();
                counter.set(current + 1);
                current
            });

            wasm_bindgen_futures::spawn_local(async move {
                let array_buffer = JsFuture::from(blob.array_buffer())
                    .await
                    .expect("Failed to read array buffer")
                    .dyn_into::<ArrayBuffer>()
                    .unwrap();
                let array = Uint8Array::new(&array_buffer);
                let data = array.to_vec();

                // Bincode is currently broken, fall back to json for now.
                // See: https://github.com/leetvr/hy/issues/189
                // let packet: net_types::ServerPacket =
                //     bincode::deserialize(&data).expect("Failed to deserialize server packet");
                let packet: net_types::ServerPacket =
                    serde_json::de::from_slice(&data).expect("Failed to deserialize server packet");

                incoming_messages
                    .borrow_mut()
                    .insert(sequence_number, packet);
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
