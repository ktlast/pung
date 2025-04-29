use crate::message::Message;
use bincode;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub async fn send_message(
    socket: Arc<UdpSocket>,
    msg: &Message,
    addr: &str,
) -> std::io::Result<()> {
    let encoded =
        bincode::encode_to_vec(msg, bincode::config::standard()).expect("Failed to encode message");
    socket.send_to(&encoded, addr).await?;
    Ok(())
}
