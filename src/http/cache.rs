use hyper::body::Bytes;
use std::{collections::BTreeMap, sync::OnceLock, sync::RwLock};

static CACHE: OnceLock<RwLock<BTreeMap<String, Cache>>> = OnceLock::new();
pub fn get_cache() -> &'static RwLock<BTreeMap<String, Cache>> {
    CACHE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

pub struct Cache {
    last_modified: u64,
    buffer: Bytes,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            last_modified: todo!(),
            buffer: todo!(),
        }
    }
}
