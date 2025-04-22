use async_trait::async_trait;
use std::io;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::{net::TcpStream, sync::Mutex};

use crate::utils::encoding::encode_string_to_base64;

#[async_trait]
pub trait MessageSink: Send + Sync {
    async fn send(&self, msg: &str) -> io::Result<()>;
}

pub struct TcpSink(pub Mutex<tokio::io::WriteHalf<TcpStream>>);

#[async_trait]
impl MessageSink for TcpSink {
    async fn send(&self, msg: &str) -> io::Result<()> {
        let mut w = self.0.lock().await;
        let encoded = encode_string_to_base64(msg);
        let bytes = encoded.as_bytes();

        w.write_u32(bytes.len() as u32).await?;
        w.write_all(&bytes).await?;
        w.flush().await
    }
}

pub async fn safe_send(sink: &Mutex<Option<Arc<dyn MessageSink>>>, msg: &str) -> io::Result<()> {
    let slot = sink.lock().await;
    if let Some(sink) = &*slot {
        sink.send(msg).await?;
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "Sink not connected",
        ))
    }
}
