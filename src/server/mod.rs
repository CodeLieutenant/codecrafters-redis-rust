use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

pub(crate) mod handler;
pub(crate) mod tcp;

pub(crate) type ArcMap = Arc<RwLock<HashMap<Box<str>, Box<str>>>>;