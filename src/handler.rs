use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

use crate::parser;

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

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.stream.read_buf(self.buf).await.map_err(Error::Read)?;
            let _command = parser::parse(self.buf)?;
            self.stream
                .write_all(b"+PONG\r\n")
                .await
                .map_err(Error::Write)?;
        }
    }
}
