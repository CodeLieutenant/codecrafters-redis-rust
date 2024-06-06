use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};

use crate::resp::Value as RespValue;

#[derive(Debug)]
pub struct Database {
    map: Map,
    handle: JoinHandle<()>,
}

type Map = Arc<RwLock<HashMap<Box<[u8]>, Entry>>>;

#[derive(Debug)]
pub enum Value {
    String(Cow<'static, str>),
    Bytes(Cow<'static, [u8]>),
    Integer(i64),
    Null,
}

#[derive(Debug)]
enum Entry {
    Expire {
        value: Value,
        created: Instant,
        duration: Duration,
    },
    NonExpire(Value),
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
        key: Cow<'a, [u8]>,
        value: impl TryInto<Value, Error=&'static str>,
        duration: Option<Duration>,
    ) {
        let mut lock = self.map.write().await;

        let _ = lock.insert(
            key.to_owned().into(),
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

    async fn clean(map: &Map) {
        let guard = map.read().await;
        let now = Instant::now();

        let keys = guard.iter()
            .filter_map(|(key, entry)| match entry {
                Entry::Expire {
                    value: _,
                    duration,
                    created,
                } if created.add(*duration).lt(&now) => Some(key.clone()),
                _ => None,
            })
            .collect::<Vec<Box<[u8]>>>();

        let mut guard = map.write().await;
        let now = Instant::now();
        for key in keys {
            match guard.get(&key) {
                Some(Entry::Expire { value: _, created, duration }) if created.add(*duration).lt(&now) => {
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

impl<'a> TryFrom<&RespValue<'a>> for Value {
    type Error = &'static str;

    fn try_from(value: &RespValue<'a>) -> Result<Self, Self::Error> {
        match value {
            RespValue::Null => Ok(Value::Null),
            RespValue::SimpleString(val) => Ok(Value::String(Cow::Owned(val.to_string()))),
            RespValue::Integer(val) => Ok(Value::Integer(*val)),
            RespValue::BulkString(val) => Ok(Value::Bytes(Cow::Owned(val.to_vec().into()))),
            _ => Err("invalid value"),
        }
    }
}


#[cfg(test)]
mod tests {

}