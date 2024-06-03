use crate::value::{Value, OK, PONG};
use std::io::ErrorKind;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tracing::instrument;

use crate::bytes::Buffer;
use crate::parser::Error as ParserError;
use crate::resp::Error as RespError;
use crate::Command;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read from connection: {0}")]
    Read(std::io::Error),

    #[error("Failed to write to connection: {0}")]
    Write(std::io::Error),

    #[error("Parser error: {0}")]
    Parse(#[from] crate::parser::Error),

    #[error("Failed to acquire data from pool")]
    AcquirePool,
}

#[derive(Debug)]
pub struct Handler {
    stream: BufWriter<TcpStream>,
    buf_pool: Arc<sharded_slab::Pool<Buffer>>,
    vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
}

impl Handler {
    #[instrument]
    pub fn new(
        stream: TcpStream,
        buf_pool: Arc<sharded_slab::Pool<Buffer>>,
        vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
    ) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buf_pool,
            vec_pool,
        }
    }

    async fn write(&mut self, output: impl AsRef<[u8]>) -> Result<(), std::io::Error> {
        self.stream.write_all(output.as_ref()).await?;

        self.stream.flush().await?;

        Ok(())
    }

    async fn write_error(&mut self, err: &dyn std::error::Error) ->  Result<(), std::io::Error> {
        let mut output = Arc::clone(&self.vec_pool)
            .create_owned()
            .ok_or(Error::AcquirePool)
            .unwrap();

        let val: Value = err.into();
        val.serialize(&mut output);
        self.write(&output as &[u8]).await?;

        Ok(())
    }

    #[instrument]
    async fn handle_command<'b>(&mut self, command: Command<'b>) -> Result<(), std::io::Error> {
        match command {
            Command::Ping => self.write(PONG).await?,
            Command::Echo(val) => {
                let mut output = Arc::clone(&self.vec_pool)
                    .create_owned()
                    .ok_or(Error::AcquirePool)
                    .unwrap();

                Value::BulkString(val).serialize(&mut output);

                self.write(&output as &[u8]).await?;
            }
            Command::Command => {}
            Command::Get(_) => {}
            Command::Set { key: _ } => self.write(OK).await?,
        };

        Ok(())
    }

    #[instrument]
    pub async fn run(&mut self) -> Result<(), std::io::Error> {
        loop {
            let mut reader = Arc::clone(&self.buf_pool)
                .create_owned()
                .ok_or(Error::AcquirePool)
                .unwrap();

            self.stream.read_buf(&mut reader.0).await?;

            let mut parser = crate::parser::Parser::parse(&reader.0);

            let command = match parser {
                Ok(ref mut parser) => parser.command(),
                Err(ParserError::Parse(RespError::Incomplete)) => continue,
                Err(err) => {
                    self.write_error(&err).await?;
                    return Err(std::io::Error::new(ErrorKind::InvalidInput, err));
                }
            };

            match command {
                Ok(command) => self.handle_command(command).await?,
                Err(err) => {
                    self.write_error(&err).await?;
                    return Err(std::io::Error::new(ErrorKind::InvalidInput, err))
                },
            }
        }
    }
}
