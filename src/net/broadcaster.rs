use crate::message::Message;
use tokio::net::UdpSocket;
use bincode;

pub async fn send_message(socket: &UdpSocket, msg: &Message, addr: &str) -> std::io::Result<()> {
    let encoded = bincode::serialize(msg).expect("Failed to encode message");
    socket.send_to(&encoded, addr).await?;
    Ok(())
}
