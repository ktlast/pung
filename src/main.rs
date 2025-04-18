mod net;
mod message;

use tokio::net::UdpSocket;
use tokio::io::{self, AsyncBufReadExt};
use net::{broadcaster, listener};
use message::Message;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let socket_send = UdpSocket::bind("0.0.0.0:8888").await?;
    socket_send.set_broadcast(true)?;
    
    let socket_recv = UdpSocket::bind("0.0.0.0:8889").await?;

    // Spawn listener
    tokio::spawn(async move {
        if let Err(e) = listener::listen(&socket_recv).await {
            eprintln!("Listen error: {:?}", e);
        }
    });

    // Read user input
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    println!("Enter messages:");

    while let Ok(Some(line)) = lines.next_line().await {
        let msg = Message {
            sender: "test".to_string(),
            content: line,
        };
        broadcaster::send_message(&socket_send, &msg, "255.255.255.255:8889").await?;
    }

    Ok(())
}
