mod value;

use std::borrow::Cow;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

pub use crate::database::value::Value;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

#[derive(Debug)]
pub struct Database {
    map: Map,
    handle: JoinHandle<()>,
}

type Map = Arc<RwLock<HashMap<Box<[u8]>, Entry>>>;

#[derive(Debug)]
enum Entry {
    Expire {
        value: Value,
        created: Instant,
        duration: Duration,
    },
    NonExpire(Value),
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        let map: Map = Arc::new(RwLock::new(HashMap::with_capacity(1024)));

        let cl = Arc::clone(&map);
        let handle: JoinHandle<()> = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(10)).await;
                Self::clean(&cl).await;
            }
        });

        Self { map, handle }
    }

    pub async fn insert<'a>(
        &self,
        key: impl Into<Cow<'a, [u8]>>,
        value: impl TryInto<Value, Error = &'static str>,
        duration: Option<Duration>,
    ) {
        let key = key.into().clone().into();
        let mut lock = self.map.write().await;

        let _ = lock.insert(
            key,
            match duration {
                Some(duration) => Entry::Expire {
                    value: value.try_into().unwrap(),
                    created: Instant::now(),
                    duration,
                },
                None => Entry::NonExpire(value.try_into().unwrap()),
            },
        );
    }

    pub async fn get_by_string(&self, key: impl AsRef<str>) -> Option<Value> {
        self.get(Cow::Borrowed(key.as_ref().as_bytes())).await
    }

    pub async fn get<'a>(&self, key: impl Into<Cow<'a, [u8]>>) -> Option<Value> {
        let (key, should_drop) = match key.into() {
            Cow::Borrowed(slice) => {
                let ptr = slice.as_ptr();
                let len = slice.len();

                unsafe {
                    let raw = std::slice::from_raw_parts_mut(ptr as *mut u8, len);
                    (ManuallyDrop::new(Box::from_raw(raw as *mut [u8])), false)
                }
            }
            Cow::Owned(vec) => (ManuallyDrop::new(vec.into_boxed_slice()), true),
        };

        let now = Instant::now();
        let guard = self.map.read().await;

        match guard.get(&key as &Box<[u8]>) {
            Some(Entry::NonExpire(val)) => {
                if should_drop {
                    ManuallyDrop::into_inner(key);
                }

                Some(val.clone())
            }
            Some(Entry::Expire {
                value: val,
                created,
                duration,
            }) if now.lt(&created.add(*duration)) => {
                if should_drop {
                    ManuallyDrop::into_inner(key);
                }

                Some(val.clone())
            }
            None => None,
            _ => None,
        }
    }

    async fn clean(map: &Map) {
        let guard = map.read().await;
        let now = Instant::now();

        let keys = guard
            .iter()
            .filter_map(|(key, entry)| match entry {
                Entry::Expire {
                    value: _,
                    duration,
                    created,
                } if created.add(*duration).lt(&now) => Some(key.clone()),
                _ => None,
            })
            .collect::<Vec<Box<[u8]>>>();

        drop(guard);
        let mut guard = map.write().await;
        let now = Instant::now();
        for key in keys {
            match guard.get(&key) {
                Some(Entry::Expire {
                    value: _,
                    created,
                    duration,
                }) if created.add(*duration).lt(&now) => {
                    guard.remove(&key);
                }
                _ => continue,
            };
        }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_non_expire() {
        let database = Database::new();

        database.insert(b"key", 1i64, None).await;

        let val = database.get(b"key").await;
        assert_eq!(Some(Value::Integer(1)), val);

        let val = database.get(b"not_exists").await;
        assert_eq!(None, val);
    }

    #[tokio::test]
    async fn test_database_get_manually_drop() {
        let database = Database::new();

        database.insert(b"key", 1i64, None).await;

        let val = database.get(Cow::Owned(b"key".to_vec())).await;
        assert_eq!(Some(Value::Integer(1)), val);

        let val = database.get(Cow::Owned(b"not_exists".to_vec())).await;
        assert_eq!(None, val);
    }

    #[tokio::test]
    async fn test_database_expired_value() {
        let database = Database::new();

        database
            .insert(b"key", 1i64, Some(Duration::from_millis(100)))
            .await;

        let val = database.get(b"key").await;
        assert_eq!(Some(Value::Integer(1)), val);

        sleep(Duration::from_millis(120)).await;

        let val = database.get(b"key").await;
        assert_eq!(None, val);
    }

    #[tokio::test]
    async fn test_database_expired_value_manual_drop() {
        let database = Database::new();

        database
            .insert(b"key", 1i64, Some(Duration::from_millis(100)))
            .await;

        let val = database.get(Cow::Owned(b"key".to_vec())).await;
        assert_eq!(Some(Value::Integer(1)), val);

        sleep(Duration::from_millis(120)).await;

        let val = database.get(Cow::Owned(b"key".to_vec())).await;
        assert_eq!(None, val);
    }

    #[tokio::test]
    async fn test_database_clean() {
        let database = Database::new();

        database
            .insert(b"key1", 1i64, Some(Duration::from_millis(10)))
            .await;

        database
            .insert(b"key2", 1i64, Some(Duration::from_millis(100)))
            .await;

        sleep(Duration::from_millis(11)).await;
        Database::clean(&database.map).await;

        assert!(database
            .map
            .read()
            .await
            .get(&b"key1".to_vec().into_boxed_slice())
            .is_none());
        assert!(database
            .map
            .read()
            .await
            .get(&b"key2".to_vec().into_boxed_slice())
            .is_some());

        sleep(Duration::from_millis(100)).await;
        Database::clean(&database.map).await;
        assert!(database
            .map
            .read()
            .await
            .get(&b"key2".to_vec().into_boxed_slice())
            .is_none());
    }
}
