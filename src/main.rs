use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

mod parser;
mod command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:6379").await?;

    loop {
        let (mut client, _socket) = listener.accept().await?;

        let mut buf = vec![0u8; 1024];
        let nread = client.read(&mut buf).await?;

        let input = std::str::from_utf8(&buf[..nread]).unwrap();
        let _command = parser::parse(input)?;

        client.write_all(b"+PONG\r\n").await?;
    }
}
