use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

use crate::Command;
use crate::error::Error as CrateError;
use crate::resp::Error as RespError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read from connection: {0}")]
    Read(std::io::Error),

    #[error("Failed to write to connection: {0}")]
    Write(std::io::Error),
}

pub struct Handler<'a> {
    stream: BufWriter<TcpStream>,
    buf: &'a mut BytesMut,
}

impl<'a> Handler<'a> {
    pub fn new(stream: TcpStream, buf: &'a mut BytesMut) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buf,
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<(), Box<dyn std::error::Error + 'static>> {
        match command {
            Command::Ping => {
                self.stream
                    .write_all(b"+PONG\r\n")
                    .await
                    .map_err(Error::Write)?;

                Ok(())
            }
            Command::Echo(_) => Err("Invalid command".into()),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + 'static>> {
        loop {
            self.stream.read_buf(self.buf).await.map_err(Error::Read)?;
            let parser = crate::parser::Parser::parse(&self.buf);

            match parser {
                Ok(parser) => {
                    self.buf.clear();
                    self.handle_command(parser.command()?).await?;
                }
                Err(CrateError::ParseError(RespError::Incomplete)) => continue,
                Err(err) => return Err(err.into()),
            }
        }
    }
}
