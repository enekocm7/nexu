use dioxus::core::anyhow;
use p2p::{ChatClient, ChatMessage, Message, Ticket};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{Mutex, OnceCell};

pub struct DesktopClient {
    client: OnceCell<Mutex<ChatClient>>,
    message_receivers: HashMap<String, UnboundedReceiver<Message>>,
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

    pub async fn peer_id(&self) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        Ok(client.lock().await.peer_id().to_string())
    }

    pub async fn create_topic(&mut self, name: &str) -> anyhow::Result<String> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;
        let ticket = client.lock().await.create_topic(name).await?;
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

    pub async fn send(&self, message: Message) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        match message {
            Message::Chat(chat_msg) => {
                client.lock().await.send(Message::Chat(chat_msg)).await?;
                Ok(())
            }
            Message::TopicMetadata(metadata) => {
                client
                    .lock()
                    .await
                    .send(Message::TopicMetadata(metadata))
                    .await?;
                Ok(())
            }
            Message::JoinTopic => {
                client.lock().await.send(Message::JoinTopic).await?;
                Ok(())
            }
            Message::LeaveTopic => {
                client.lock().await.send(Message::LeaveTopic).await?;
                Ok(())
            }
        }
    }

    pub async fn get_chat_message(
        &self,
        ticket_str: &str,
        message: &str,
    ) -> anyhow::Result<ChatMessage> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = ChatMessage::new(
            *client.lock().await.peer_id(),
            message.to_string(),
            timestamp,
            ticket.topic,
        );
        Ok(message)
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

    pub fn get_message_receiver(&mut self) -> &mut HashMap<String, UnboundedReceiver<Message>> {
        &mut self.message_receivers
    }
}
