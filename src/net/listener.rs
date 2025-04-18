use crate::message::Message;
use crate::utils;
use tokio::net::UdpSocket;
use bincode;

pub async fn listen(socket: &UdpSocket) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
            let formatted_time = utils::display_time_from_timestamp(msg.timestamp);
            println!("[{}]: {}     ({})", msg.sender, msg.content, formatted_time);
        } else {
            eprintln!("Received invalid message from {}", addr);
        }
    }
}
