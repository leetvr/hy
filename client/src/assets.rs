use {
    anyhow::{format_err, Result},
    std::{cell::RefCell, collections::HashMap, rc::Rc},
    tokio::sync::oneshot,
    wasm_bindgen::{prelude::Closure, JsCast},
    web_sys::{js_sys::Uint8Array, XmlHttpRequest, XmlHttpRequestResponseType},
};

pub struct Assets {
    pending: HashMap<String, oneshot::Receiver<Result<AssetData>>>,
    loaded: HashMap<String, Option<AssetData>>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            pending: Default::default(),
            loaded: Default::default(),
        }
    }

    pub fn get(&mut self, path: &str) -> Option<&Vec<u8>> {
        if let Some(data) = self.loaded.get(path) {
            return data.as_ref();
        }

        if self.pending.contains_key(path) {
            return None;
        }

        let rx = fetch_asset(path.to_string());
        self.pending.insert(path.to_string(), rx);

        None
    }

    pub fn maintain(&mut self) {
        // self.pending.retain(|name, rx| {
        //     if let Some(data) = rx.try_recv().ok() {
        //         let data = match data {
        //             Ok(data) => Some(data),
        //             Err(err) => {
        //                 tracing::error!("Failed to load asset {name}: {err}");
        //                 None
        //             }
        //         };
        //         self.loaded.insert(name.clone(), data);
        //         false
        //     } else {
        //         true
        //     }
        // });
    }
}

type AssetData = Vec<u8>;

fn fetch_asset(path: String) -> oneshot::Receiver<Result<AssetData>> {
    let req = XmlHttpRequest::new().expect("Failed to create XmlHttpRequest");
    req.open("GET", &format!("/{path}"))
        .expect("Failed to open request");
    req.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
    let (tx, rx) = oneshot::channel();

    let f = Rc::new(RefCell::new(None));
    *f.borrow_mut() = Some(Closure::wrap(Box::new({
        let req = req.clone();
        let tx = RefCell::new(Some(tx));
        let f = f.clone();
        move || {
            tracing::info!("Request complete");
            let res = if req.status() == Ok(200) {
                let data = req.response().unwrap();
                let data = Uint8Array::new(&data);
                Ok(data.to_vec())
            } else {
                let err = req.status_text();
                Err(format_err!("Request failed: {err:?}"))
            };
            let _ = tx.take().unwrap().send(res);

            f.borrow_mut().take().unwrap();
        }
    }) as Box<dyn FnMut()>));

    req.set_onload(Some(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()));

    req.send().expect("Failed to send request");
    rx
}
