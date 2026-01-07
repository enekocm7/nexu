use crate::messages::DmMessageTypes;
use flume::Sender;
use iroh::EndpointId;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const DM_ALPN: &[u8] = b"nexu/dm/0";

#[derive(Debug, Clone)]
pub struct DMProtocol {
    pub tx: Sender<(EndpointId, DmMessageTypes)>,
}

impl ProtocolHandler for DMProtocol {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let tx = self.tx.clone();
        Box::pin(async move {
            let (_send, mut recv) = connection.accept_bi().await?;
            let remote_id = connection.remote_id();

            tokio::spawn(async move {
                loop {
                    match read_frame(&mut recv).await {
                        Ok(Some(msg)) => {
                            if tx.send((remote_id, msg)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("Error reading frame: {:?}", e);
                            break;
                        }
                    }
                }
            });

            Ok(())
        })
    }
}

pub async fn write_frame(stream: &mut SendStream, message: &[u8]) -> anyhow::Result<()> {
    let len = message.len() as u32;
    stream.write_u32(len).await?;
    stream.write_all(message).await?;

    Ok(())
}

async fn read_frame(stream: &mut RecvStream) -> anyhow::Result<Option<DmMessageTypes>> {
    let Ok(frame_len) = stream.read_u32().await else {
        return Ok(None);
    };

    let mut buf = vec![0u8; frame_len as usize];
    stream.read_exact(&mut buf).await?;

    let message = postcard::from_bytes(&buf)?;
    Ok(Some(message))
}
