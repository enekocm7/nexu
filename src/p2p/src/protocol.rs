use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};

pub const DM_ALPN: &[u8] = b"nexu/dm/0";

#[derive(Debug, Clone)]
pub struct DMProtocol;

impl ProtocolHandler for DMProtocol {
    fn accept(
        &self,
        connection: Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        Box::pin(async move {
            let (mut send, mut recv) = connection.accept_bi().await?;

            tokio::io::copy(&mut recv, &mut send).await?;

            send.finish()?;

            connection.closed().await;
            Ok(())
        })
    }
}
