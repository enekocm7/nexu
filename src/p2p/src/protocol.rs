//! # Direct Messaging Protocol
//!
//! This module defines the protocol handler and utilities for the Direct Messaging (DM)
//! system in Nexu. It handles the low-level details of accepting connections,
//! reading/writing frames, and dispatching incoming messages to the application via a channel.

use crate::messages::DmMessageTypes;
use flume::Sender;
use iroh::EndpointId;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The Application-Layer Protocol Negotiation (ALPN) string used for Nexu Direct Messages.
///
/// This string identifies the protocol version `nexu/dm/0`.
pub const DM_ALPN: &[u8] = b"nexu/dm/0";

/// The protocol handler for Direct Messages.
///
/// This struct implements [`ProtocolHandler`], allowing it to be registered with the iroh Router.
/// When a peer connects using `DM_ALPN`, the `accept` method is called.
#[derive(Debug, Clone)]
pub struct DMProtocol {
    /// Channel sender to forward received messages to the main application logic.
    pub tx: Sender<(EndpointId, DmMessageTypes)>,
}

impl ProtocolHandler for DMProtocol {
    /// Accepts an incoming connection for the DM protocol.
    ///
    /// This method spawns a task to continuously read messages from the incoming bidirectional stream.
    /// Received messages are deserialized and sent through the `tx` channel along with the sender's `EndpointId`.
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
                            eprintln!("Error reading frame: {e}");
                            break;
                        }
                    }
                }
            });

            Ok(())
        })
    }
}

/// Writes a length-prefixed message frame to the stream.
///
/// 1. Writes the length of the message as a `u32` (4 bytes).
/// 2. Writes the message bytes.
///
/// # Arguments
///
/// * `stream` - The output stream to write to.
/// * `message` - The byte slice containing the serialized message.
/// 
/// # Errors
/// 
/// If the message length cant be parsed to a u32 or if it cant write to the stream
pub async fn write_frame(stream: &mut SendStream, message: &[u8]) -> anyhow::Result<()> {
    let len = u32::try_from(message.len())?;
    stream.write_u32(len).await?;
    stream.write_all(message).await?;

    Ok(())
}

/// Reads a length-prefixed message frame from the stream.
///
/// 1. Reads a `u32` to determine the message length.
/// 2. Reads that many bytes into a buffer.
/// 3. Deserializes the buffer into a [`DmMessageTypes`] enum using `postcard`.
///
/// Returns `Ok(None)` if the stream has closed (EOF) while trying to read the length.
async fn read_frame(stream: &mut RecvStream) -> anyhow::Result<Option<DmMessageTypes>> {
    let Ok(frame_len) = stream.read_u32().await else {
        return Ok(None);
    };

    let mut buf = vec![0u8; frame_len as usize];
    stream.read_exact(&mut buf).await?;

    let message = postcard::from_bytes(&buf)?;
    Ok(Some(message))
}
