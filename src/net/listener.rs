use crate::message::Message;
use tokio::net::UdpSocket;
use bincode;

pub async fn listen(socket: &UdpSocket) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
            println!("[{}]: {}", msg.sender, msg.content);
        } else {
            eprintln!("Received invalid message from {}", addr);
        }
    }
}
