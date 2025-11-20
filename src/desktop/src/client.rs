use dioxus::core::anyhow;
use p2p::{ChatClient, Ticket};
use std::str::FromStr;
use tokio::sync::{Mutex, OnceCell};

pub struct DesktopClient {
    client: OnceCell<Mutex<ChatClient>>,
}

impl DesktopClient {
    pub fn new() -> Self {
        Self {
            client: OnceCell::new(),
        }
    }

    pub async fn initialize(&self) -> anyhow::Result<()> {
        self.client
            .get_or_try_init(|| async { ChatClient::new().await.map(Mutex::new) })
            .await?;
        Ok(())
    }

    pub async fn peer_id(&self) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        Ok(client.lock().await.peer_id().to_string())
    }

    pub async fn create_topic(&self) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let topic_id = client.lock().await.create_topic().await?;
        Ok(topic_id.to_string())
    }

    pub async fn join_topic(&self, topic_id: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        client.lock().await.join_topic_from_string(topic_id).await?;
        Ok(())
    }

    pub async fn send_message(&self, topic_id: &str, message: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let ticket = Ticket::from_str(topic_id)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        client
            .lock()
            .await
            .send_message(message, timestamp, &ticket.topic)
            .await?;
        Ok(())
    }
}
