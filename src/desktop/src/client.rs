use dioxus::core::anyhow;
use flume::Receiver;
use p2p::messages::DmMessageTypes;
use p2p::types::Address;
use p2p::{ChatClient, ChatMessage, MessageTypes, Ticket};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::{Mutex, OnceCell};

pub struct DesktopClient {
    client: OnceCell<Mutex<ChatClient>>,
    message_receivers: HashMap<String, Receiver<MessageTypes>>,
    dm_receivers: HashMap<String, Receiver<DmMessageTypes>>,
}

impl DesktopClient {
    pub fn new() -> Self {
        Self {
            client: OnceCell::new(),
            message_receivers: HashMap::new(),
            dm_receivers: HashMap::new(),
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
    ) -> anyhow::Result<ChatMessage> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let ticket = Ticket::from_str(ticket_str)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let message = ChatMessage::new(
            client.lock().await.peer_id(),
            message.to_string(),
            timestamp,
            ticket.topic,
        );
        Ok(message)
    }

    pub async fn get_dm_chat_message(
        &self,
        address_str: &str,
        message: &str,
    ) -> anyhow::Result<p2p::messages::DmChatMessage> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let address = Address::from_str(address_str)?;
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let dm_message = p2p::messages::DmChatMessage::new(
            client.lock().await.peer_id(),
            address.into(),
            message.to_string(),
            timestamp,
        );

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

    pub async fn connect_to_user(&mut self, username: &str) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let address = Address::from_str(username)?;

        client.lock().await.connect_peer(address).await?;

        Ok(())
    }

    pub async fn send_dm(&self, address: &str, message: DmMessageTypes) -> anyhow::Result<()> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        let endpoint_addr = Address::from_str(address)?;

        client.lock().await.send_dm(endpoint_addr, message).await?;

        Ok(())
    }

    pub fn get_dm_receiver(&mut self) -> &mut HashMap<String, Receiver<DmMessageTypes>> {
        &mut self.dm_receivers
    }

    pub async fn get_self_address(&self) -> anyhow::Result<Address> {
        let client = self
            .client
            .get()
            .ok_or_else(|| anyhow!("Client is not initialized"))?;

        Ok(Address::from(client.lock().await.endpoint_addr()))
    }
}
