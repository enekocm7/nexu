use std::str::FromStr;
use dioxus::core::anyhow;
use p2p::{ChatClient, Ticket};
use std::sync::OnceLock;
use tokio::sync::Mutex;

pub struct DesktopClient {
    client: Mutex<OnceLock<ChatClient>>,
}

impl DesktopClient {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(OnceLock::new()),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        let cell = self.client.lock().await;

        if cell.get().is_some() {
            return Err(anyhow!("Client is already initialized"));
        }
        let chat_client = ChatClient::new().await?;
        cell.set(chat_client).map_err(|_| anyhow!("Failed to set ChatClient"))?;

        Ok(())
    }

    pub async fn peer_id(&self) -> anyhow::Result<String> {
        let cell = self.client.lock().await;
        let client = cell.get().ok_or_else(|| anyhow!("Client is not initialized"))?;
        Ok(client.peer_id().to_string())
    }

    pub async fn create_topic(&self) -> anyhow::Result<String> {
        let mut cell = self.client.lock().await;
        let client = cell.get_mut().ok_or_else(|| anyhow!("Client is not initialized"))?;
        let topic_id = client.create_topic().await?;
        Ok(topic_id.to_string())
    }

    pub async fn join_topic(&self, topic_id: &str) -> anyhow::Result<()> {
        let mut cell = self.client.lock().await;
        let client = cell.get_mut().ok_or_else(|| anyhow!("Client is not initialized"))?;
        client.join_topic_from_string(topic_id).await?;
        Ok(())
    }

    pub async fn send_message(&self, topic_id: &str, message: &str) -> anyhow::Result<()> {
        let mut cell = self.client.lock().await;
        let client = cell.get_mut().ok_or_else(|| anyhow!("Client is not initialized"))?;
        let ticket = Ticket::from_str(topic_id)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        client.send_message(message, timestamp, &ticket.topic).await?;
        Ok(())
    }
}