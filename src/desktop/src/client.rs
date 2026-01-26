use dioxus::core::anyhow;
use flume::Receiver;
use futures_lite::Stream;
use p2p::messages::DmMessageTypes;
use p2p::{
    AddProgressItem, BlobTicket, ChatClient, DownloadProgress, EndpointId,
    MessageTypes, Ticket,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;
use tokio::sync::{Mutex, OnceCell};

pub struct DesktopClient {
    client: OnceCell<Mutex<ChatClient>>,
    message_receivers: HashMap<String, Receiver<MessageTypes>>,
}

impl DesktopClient {
    pub fn new() -> Self {
        Self {
            client: OnceCell::new(),
            message_receivers: HashMap::new(),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        let dir = dirs::data_dir()
            .ok_or_else(|| anyhow!("Could not find data directory"))?
            .join("nexu");
        self.client
            .get_or_try_init(|| async { ChatClient::new(dir).await.map(Mutex::new) })
            .await?;
        Ok(())
    }

    pub async fn peer_id(&self) -> anyhow::Result<EndpointId> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        Ok(client.lock().await.peer_id())
    }

    pub async fn create_topic(&mut self) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let ticket = client.lock().await.create_topic().await?;
        let message_receiver = client.lock().await.listen(&ticket.topic)?;
        let ticket_str = ticket.to_string();
        self.message_receivers
            .insert(ticket_str.clone(), message_receiver);
        Ok(ticket_str)
    }

    pub async fn join_topic(&mut self, ticket_str: &str) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        let topic_id = client.lock().await.join_topic(ticket).await?;

        let message_receiver = client.lock().await.listen(&topic_id)?;

        self.message_receivers
            .insert(ticket_str.to_string(), message_receiver);

        Ok(ticket_str.to_string())
    }

    pub async fn send(&self, message: MessageTypes) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        client.lock().await.send(message).await?;
        Ok(())
    }

    pub async fn get_chat_message(
        &self,
        ticket_str: &str,
        message: &str,
    ) -> anyhow::Result<p2p::ChatMessage> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let client_guard = client.lock().await;
        let sender = client_guard.peer_id();

        let message = p2p::ChatMessage::new(sender, message.to_string(), timestamp, ticket.topic);
        Ok(message)
    }

    pub async fn get_dm_chat_message(
        &self,
        id: &str,
        message: &str,
    ) -> anyhow::Result<p2p::messages::DmChatMessage> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let client_guard = client.lock().await;
        let sender = client_guard.peer_id();
        let endpoint_id = id.parse::<EndpointId>()?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        let dm_message =
            p2p::messages::DmChatMessage::new(sender, endpoint_id, message.to_string(), timestamp);

        Ok(dm_message)
    }

    pub async fn leave_topic(&mut self, ticket_str: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        client.lock().await.leave_topic(&ticket.topic).await?;

        self.message_receivers.remove(ticket_str);

        Ok(())
    }

    pub fn get_message_receiver(&mut self) -> &mut HashMap<String, Receiver<MessageTypes>> {
        &mut self.message_receivers
    }

    pub async fn get_global_dm_receiver(
        &self,
    ) -> anyhow::Result<Receiver<(EndpointId, DmMessageTypes)>> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client not initialized"))?;
        let guard = client.lock().await;
        Ok(guard.incoming_dms())
    }

    pub async fn connect_to_user(&mut self, id: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let peer_id = id.parse::<EndpointId>()?;

        client.lock().await.connect_peer(peer_id).await?;

        Ok(())
    }

    pub async fn send_dm(&self, id: &str, message: DmMessageTypes) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let peer_id = id.parse::<EndpointId>()?;

        client.lock().await.send_dm(peer_id, message).await?;

        Ok(())
    }

    pub async fn save_blob(
        &self,
        blob: Vec<u8>,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = AddProgressItem> + Send>>> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let mut guard = client.lock().await;
        let progress = guard.save_blob(blob.as_slice());
        let stream = progress.stream().await;
        Ok(Box::pin(stream))
    }

    pub async fn save_blob_from_path(
        &self,
        blob_path: PathBuf,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = AddProgressItem> + Send>>> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let mut guard = client.lock().await;
        let progress = guard.save_blob_from_path(&blob_path);
        let stream = progress.stream().await;
        Ok(Box::pin(stream))
    }

    pub async fn download_blob(
        &self,
        blob_ticket: &BlobTicket,
    ) -> anyhow::Result<DownloadProgress> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let mut guard = client.lock().await;
        Ok(guard.download_blob(blob_ticket))
    }

    pub async fn get_blob_path(
        &self,
        hash: impl Into<p2p::Hash>,
        extension: impl AsRef<OsStr>,
    ) -> anyhow::Result<PathBuf> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let guard = client.lock().await;
        guard.get_blob_path(hash, extension).await
    }
}
