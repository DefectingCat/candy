use hyper::body::Bytes;

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
