use crate::message::Message;
use tokio::net::UdpSocket;
use bincode;
use std::sync::Arc;

pub async fn send_message(socket:Arc<UdpSocket>, msg: &Message, addr: &str) -> std::io::Result<()> {
    let encoded = bincode::serialize(msg).expect("Failed to encode message");
    socket.send_to(&encoded, addr).await?;
    Ok(())
}
