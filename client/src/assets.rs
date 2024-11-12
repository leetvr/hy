use {
    blocks::BlockRegistry,
    entities::EntityData,
    itertools::Itertools,
    std::{
        cell::{Ref, RefCell},
        collections::{HashMap, HashSet},
        rc::Rc,
    },
};

pub struct Assets {
    pending: HashSet<String>,
    loaded: Rc<RefCell<HashMap<String, AssetData>>>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            pending: Default::default(),
            loaded: Default::default(),
        }
    }

    pub fn load_block_textures(&mut self, block_registry: &BlockRegistry) {
        for block_type in block_registry.iter() {
            self.get_or_load(&block_type.top_texture);
            self.get_or_load(&block_type.bottom_texture);
            self.get_or_load(&block_type.east_texture);
            self.get_or_load(&block_type.west_texture);
            self.get_or_load(&block_type.north_texture);
            self.get_or_load(&block_type.south_texture);
        }
    }

    pub fn load_entity_models(&mut self, entities: &[EntityData]) {
        for model_name in entities.iter().map(|e| &e.model_path).unique() {
            self.get_or_load(model_name);
        }
    }

    pub fn get_or_load(&mut self, path: &str) -> Option<Ref<Vec<u8>>> {
        // Do we have this asset loaded already?
        if self.loaded.borrow().contains_key(path) {
            return Some(Ref::map(self.loaded.borrow(), |loaded| &loaded[path]));
        }

        // Are we already trying to load the asset?
        if self.pending.contains(path) {
            tracing::debug!("{path:} is pending, ignoring");
            return None;
        }

        // Nope! Okay, so now let's load it. First, let's mark it in our pending list..
        self.pending.insert(path.to_string());

        tracing::debug!("Fetching {path:}..");

        let path = path.to_string();
        let loaded = self.loaded.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let data = gloo_net::http::Request::get(&path)
                .send()
                .await
                .unwrap()
                .binary()
                .await
                .unwrap();
            tracing::debug!("Fetched {path} successfully!");
            loaded.borrow_mut().insert(path, data);
        });

        None
    }

    pub fn get(&self, path: &str) -> Option<Ref<Vec<u8>>> {
        if self.loaded.borrow().contains_key(path) {
            Some(Ref::map(self.loaded.borrow(), |loaded| &loaded[path]))
        } else {
            None
        }
    }
}

type AssetData = Vec<u8>;
