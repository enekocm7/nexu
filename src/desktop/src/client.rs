use dioxus::core::anyhow;
use p2p::{ChatClient, ChatMessage, Ticket};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{Mutex, OnceCell};

pub struct DesktopClient {
    client: OnceCell<Mutex<ChatClient>>,
    message_receivers: HashMap<String, UnboundedReceiver<ChatMessage>>,
}

impl DesktopClient {
    pub fn new() -> Self {
        Self {
            client: OnceCell::new(),
            message_receivers: HashMap::new(),
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

    pub async fn create_topic(&mut self) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let ticket = client.lock().await.create_topic().await?;
        let message_receiver = client.lock().await.listen(&ticket.topic).await?;
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

        let message_receiver = client.lock().await.listen(&topic_id).await?;
        
        self.message_receivers
            .insert(ticket_str.to_string(), message_receiver);

        Ok(ticket_str.to_string())
    }

    pub async fn send_message(&self, ticket_str: &str, message: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;

        client
            .lock()
            .await
            .send_message(message, timestamp, &ticket.topic)
            .await?;
        Ok(())
    }

    pub fn get_message_receiver(
        &mut self
    ) -> &mut HashMap<String, UnboundedReceiver<ChatMessage>> {
        &mut self.message_receivers
    }
}
