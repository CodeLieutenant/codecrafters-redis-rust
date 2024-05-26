use std::ops::{Deref, DerefMut};

use bytes::BytesMut;
use sharded_slab::Clear;

const CAPACITY: usize = 64 * 1024;

#[derive(Debug)]
pub(super) struct Buffer(BytesMut);

impl Clear for Buffer {
    fn clear(&mut self) {
        self.0.clear();
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self(BytesMut::with_capacity(CAPACITY))
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Buffer {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}