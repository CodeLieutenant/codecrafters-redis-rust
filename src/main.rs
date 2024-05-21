use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod command;
mod parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:6379").await?;

    loop {
        let (client, _socket) = listener.accept().await?;

        tokio::spawn(async move {
            match handle_client(client).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                }
            }
        });
    }
}

async fn handle_client(mut client: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = vec![0u8; 65536];

    loop {
        let nread = client.read(&mut buf).await?;

        let input = std::str::from_utf8(&buf[..nread]).unwrap();
        let _command = parser::parse(input)?;

        client.write_all(b"+PONG\r\n").await?;
    }
}
