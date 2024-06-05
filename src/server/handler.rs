use std::borrow::Cow;
use crate::value::{Value, OK, PONG};
use std::io::{ErrorKind, Result as IoResult, Error as IoError};
use std::sync::Arc;
use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

use crate::bytes::Buffer;
use crate::parser::{Error as ParserError, Parser};
use crate::resp::Error as RespError;
use crate::Command;
use crate::server::ArcMap;

#[derive(Debug)]
pub struct Handler<W> {
    stream: BufWriter<W>,
    buf_pool: Arc<sharded_slab::Pool<Buffer>>,
    vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
}

#[derive(thiserror::Error, Debug)]
enum ClientError {
    #[error("key does not exist")]
    KeyNotExists
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] IoError),

    #[error("Check Again")]
    Again,
}

impl<W: AsyncRead + AsyncWrite + Unpin> Handler<W> {
    pub fn new(
        stream: W,
        buf_pool: Arc<sharded_slab::Pool<Buffer>>,
        vec_pool: Arc<sharded_slab::Pool<Vec<u8>>>,
    ) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buf_pool,
            vec_pool,
        }
    }

    async fn write(&mut self, output: impl AsRef<[u8]>) -> IoResult<()> {
        self.stream.write_all(output.as_ref()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn write_error(&mut self, err: &(dyn std::error::Error + Send + Sync)) -> IoResult<()> {
        let mut output = Arc::clone(&self.vec_pool)
            .create_owned()
            .ok_or_else(|| std::io::Error::new(ErrorKind::Other, "Failed to acquire vec_pool"))?;

        let val: Value = err.into();
        val.serialize(&mut output);
        self.write(&output as &[u8]).await?;

        Ok(())
    }


    async fn handle_command<'b>(&mut self, command: Command<'b>, map: ArcMap) -> IoResult<()> {
        match command {
            Command::Ping => self.write(PONG).await?,
            Command::Echo(val) => {
                let mut output = Arc::clone(&self.vec_pool).
                    create_owned().
                    ok_or_else(|| {
                        IoError::new(ErrorKind::Other, "Failed to acquire vec_pool")
                    })?;

                Value::BulkString(val).serialize(&mut output);
                self.write(&output as &[u8]).await?;
            }
            Command::Command => self.write(OK).await?,
            Command::Get(key) => {
                {
                    let guard = map.read().await;

                    if let Some(val) = guard.get(key.as_ref()) {
                        let mut output = Arc::clone(&self.vec_pool).
                            create_owned().
                            ok_or_else(|| {
                                IoError::new(ErrorKind::Other, "Failed to acquire vec_pool")
                            })?;

                        Value::BulkString(Cow::Borrowed(val)).serialize(&mut output);
                        self.write(&output as &[u8]).await?;
                        return Ok(());
                    }
                }

                self.write_error(&ClientError::KeyNotExists).await?;
            }
            Command::Set { key, value, expiration_ms: _ } => {
                {
                    map.write().await.insert(key.into(), value.into());
                }

                self.write(OK).await?
            }
        };

        Ok(())
    }

    async fn handle(&mut self, map: &ArcMap, mut reader: &mut BytesMut) -> Result<(), Error> {
        self.stream.read_buf(&mut reader).await?;

        let mut parser = Parser::parse(&reader);

        let command = match parser {
            Ok(ref mut parser) => parser.command(),
            Err(ParserError::Parse(RespError::Incomplete)) => return Err(Error::Again),
            Err(err) => {
                self.write_error(&err).await?;
                return Err(Error::IoError(IoError::new(ErrorKind::InvalidInput, err)));
            }
        };

        match command {
            Ok(command) => self.handle_command(command, Arc::clone(&map)).await?,
            Err(err @ ParserError::NotExists) => {
                self.write_error(&err).await?;
                return Err(Error::Again);
            }
            Err(err) => {
                self.write_error(&err).await?;
                return Err(Error::IoError(IoError::new(ErrorKind::InvalidInput, err)));
            }
        }

        Ok(())
    }

    pub async fn run(&mut self, map: &ArcMap) -> Result<(), Error> {
        let mut reader = Arc::clone(&self.buf_pool).
            create_owned().
            ok_or_else(|| {
                IoError::new(ErrorKind::Other, "Failed to buf_pool acquire pool")
            })?;

        while let Err(err) = self.handle(map, &mut reader.0).await {
            match err {
                Error::IoError(io) => return Err(Error::IoError(io)),
                Error::Again => continue
            }
        }

        Ok(())
    }
}
