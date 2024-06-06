use std::io::ErrorKind;
use std::sync::Arc;

use crate::bytes::Buffer;
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{error, info, instrument, span, Level};
use crate::Database;

use super::handler::Handler;

pub(crate) struct Server {
    listener: TcpListener,
    connection_limit: Arc<Semaphore>,
    buf_pool: Arc<sharded_slab::Pool<Buffer>>,
    vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
}

impl Server {
    #[instrument]
    #[inline]
    pub async fn new(port: u16, connection_limit: usize) -> Result<Self, io::Error> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        listener.set_ttl(60)?;

        Ok(Self {
            listener,
            connection_limit: Semaphore::new(connection_limit).into(),
            buf_pool: sharded_slab::Pool::new().into(),
            vec_pool: sharded_slab::Pool::new().into(),
        })
    }

    async fn accept_client(&self, token: OwnedSemaphorePermit, map: Arc<Database>) -> Result<(), io::Error> {
        let (client, socket) = self.listener.accept().await?;
        let span = span!(Level::INFO, "new client", addr = ?socket.ip(), port = socket.port());
        let _enter = span.enter();

        let mut handler = Handler::new(
            client,
            Arc::clone(&self.buf_pool),
            Arc::clone(&self.vec_pool),
        );

        tokio::spawn(async move {
            loop {
                if let Err(err) = handler.run(&map).await {
                    error!(err = ?err, "Failed to handle client");
                    drop(handler);
                    drop(token);
                    return;
                }
            }
        });

        Ok(())
    }

    pub async fn start(&self, db:Arc<Database>) -> Result<(), io::Error> {
        let span = span!(Level::TRACE, "Client Accept Loop");
        let _enter = span.enter();


        info!("Starting Accept connection loop");

        loop {
            let token = Arc::clone(&self.connection_limit)
                .acquire_owned()
                .await
                .map_err(|err| io::Error::new(ErrorKind::ConnectionRefused, err))?;

            match self.accept_client(token, Arc::clone(&db)).await {
                Ok(_) => info!("New Client accepted"),
                Err(err) => error!(err = ?err, "Failed to accept new client"),
            };
        }
    }
}
