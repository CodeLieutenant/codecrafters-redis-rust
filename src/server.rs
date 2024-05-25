use std::ops::DerefMut;
use std::sync::Arc;

use bytes::BytesMut;
use sharded_slab::Clear;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::BufWriter;
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, instrument};

use crate::handler::Handler;
use crate::parser;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to start TCP Server: {0}")]
    ListenerFailed(io::Error),

    #[error("Failed to accept client: {0}")]
    AcceptClient(io::Error),

    #[error(transparent)]
    Any(Box<dyn std::error::Error>),
}

#[derive(Debug)]
struct BytesMutWrapper(BytesMut);

impl Clear for BytesMutWrapper {
    fn clear(&mut self) {
        self.0.clear();
    }
}

impl Default for BytesMutWrapper {
    fn default() -> Self {
        Self(BytesMut::with_capacity(64 * 1024))
    }
}

#[derive(Debug)]
pub struct RedisServer {
    listener: TcpListener,
    connection_limit: Arc<Semaphore>,
    pool: Arc<sharded_slab::Pool<BytesMutWrapper>>,
}

impl RedisServer {
    #[instrument]
    #[inline]
    pub async fn new(port: u16, connection_limit: usize) -> Result<Self, Error> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(Error::ListenerFailed)?;

        Ok(Self {
            listener,
            connection_limit: Semaphore::new(connection_limit).into(),
            pool: sharded_slab::Pool::new().into(),
        })
    }

    #[instrument]
    async fn accept_client(&self, token: OwnedSemaphorePermit) -> Result<(), Error> {
        let (client, _socket) = self.listener.accept().await.map_err(Error::AcceptClient)?;

        let pool = Arc::clone(&self.pool);
        tokio::spawn(async move {
            let mut item = pool.create_owned().unwrap();
            let mut handler = Handler::new(client, &mut item.0);

            handler.run().await.unwrap();

            drop(handler);
            drop(token);
        });

        Ok(())
    }

    #[instrument]
    pub async fn run(&self) -> Result<(), Error> {
        info!("Starting Accept connection loop");

        loop {
            let token = Arc::clone(&self.connection_limit)
                .acquire_owned()
                .await
                .map_err(|err| Error::Any(err.into()))?;

            match self.accept_client(token).await {
                Ok(_) => info!("New Client accepted"),
                Err(err) => error!(err = ?err, "Failed to accept new client"),
            };
        }
    }
}
