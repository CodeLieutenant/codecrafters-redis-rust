use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use crate::value::Value;

use crate::{bulk_string_rc, Command, simple_string};
use crate::parser::Error as ParserError;
use crate::resp::Error as RespError;


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to read from connection: {0}")]
    Read(std::io::Error),

    #[error("Failed to write to connection: {0}")]
    Write(std::io::Error),

    #[error("Parser error: {0}")]
    Parse(#[from] crate::parser::Error)
}

pub struct Handler<'a> {
    stream: BufWriter<TcpStream>,
    reader: &'a mut BytesMut,
    output: &'a mut Vec<u8>,
}

impl<'a> Handler<'a> {
    pub fn new(stream: TcpStream, reader: &'a mut BytesMut, output: &'a mut Vec<u8>) -> Self {
        Self {
            stream: BufWriter::new(stream),
            reader,
            output,
        }
    }

    async fn handle_command(&mut self, command: Command) -> Result<(), Error> {
        self.output.clear();

        match command {
            Command::Ping => {
                simple_string!(b"PONG").serialize(self.output);
            }
            Command::Echo(val) => {
                bulk_string_rc!(&val).serialize(self.output);
            }
        }

        self.stream
            .write_all(self.output)
            .await
            .map_err(Error::Write)?;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            self.stream.read_buf(self.reader).await.map_err(Error::Read)?;
            let parser = crate::parser::Parser::parse(&self.reader);

            match parser {
                Ok(parser) => {
                    self.reader.clear();
                    self.handle_command(parser.command()?).await?;
                }
                Err(ParserError::ParseError(RespError::Incomplete)) => continue,
                Err(err) => return Err(err.into()),
            }
        }
    }
}
