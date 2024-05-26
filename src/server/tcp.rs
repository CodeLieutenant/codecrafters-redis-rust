use std::sync::Arc;

use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::{AcquireError, OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, instrument};

use super::handler::Handler;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to start TCP Server: {0}")]
    ListenerFailed(io::Error),

    #[error("Failed to accept client: {0}")]
    AcceptClient(io::Error),

    #[error("Failed to acquire token to accept new client: {0}")]
    ClientsAcquireToken(#[from] AcquireError),

    #[error("Client error: {0}")]
    Client(#[from] crate::server::handler::Error),

    #[error("Failed to acquire data from pool")]
    AcquirePool,
}

#[derive(Debug)]
pub(crate) struct Server {
    listener: TcpListener,
    connection_limit: Arc<Semaphore>,
    buf_pool: Arc<sharded_slab::Pool<super::bytes::Buffer>>,
    vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
}

impl Server {
    #[instrument]
    #[inline]
    pub async fn new(port: u16, connection_limit: usize) -> Result<Self, Error> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(Error::ListenerFailed)?;

        Ok(Self {
            listener,
            connection_limit: Semaphore::new(connection_limit).into(),
            buf_pool: sharded_slab::Pool::new().into(),
            vec_pool: sharded_slab::Pool::new().into(),
        })
    }

    #[instrument]
    async fn accept_client(&self, token: OwnedSemaphorePermit) -> Result<(), Error> {
        let (client, _socket) = self.listener.accept().await.map_err(Error::AcceptClient)?;

        let pool = Arc::clone(&self.buf_pool);
        let output_pool = Arc::clone(&self.vec_pool);
        tokio::spawn(async move {
            let mut item = pool.create_owned().ok_or(Error::AcquirePool)?;
            let mut output = output_pool.create_owned().ok_or(Error::AcquirePool)?;
            let mut handler = Handler::new(client, &mut item, &mut output);

            match handler.run().await {
                Ok(_) => info!("Closing client"),
                Err(err) => error!(err = ?err, "Failed to handle client"),
            }

            drop(handler);
            drop(token);
            Ok::<(), Error>(())
        });

        Ok(())
    }

    #[instrument]
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting Accept connection loop");

        loop {
            let token = Arc::clone(&self.connection_limit).acquire_owned().await?;

            match self.accept_client(token).await {
                Ok(_) => info!("New Client accepted"),
                Err(err) => error!(err = ?err, "Failed to accept new client"),
            };
        }
    }
}
