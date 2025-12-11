use futures_lite::StreamExt;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::{ALPN, net::Gossip, proto::TopicId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio::time::sleep;

pub enum Message {
    Chat(ChatMessage),
    JoinTopic(JoinMessage),
    LeaveTopic,
    TopicMetadata(TopicMetadataMessage),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinMessage {
    pub topic: TopicId,
}

impl JoinMessage {
    pub fn new(topic: TopicId) -> Self {
        JoinMessage { topic }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMetadataMessage {
    pub topic: TopicId,
    pub name: String,
    pub avatar_url: Option<String>,
    pub timestamp: u64,
}

impl TopicMetadataMessage {
    pub fn new(topic: TopicId, name: &str, avatar_url: Option<String>, timestamp: u64) -> Self {
        TopicMetadataMessage {
            topic,
            name: name.to_string(),
            avatar_url,
            timestamp,
        }
    }

    pub fn new_from_string(
        topic: &str,
        name: &str,
        avatar_url: Option<String>,
        timestamp: u64,
    ) -> Self {
        let topic = TopicId::from_str(topic).expect("Invalid topic ID string");
        TopicMetadataMessage {
            topic,
            name: name.to_string(),
            avatar_url,
            timestamp,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: EndpointId,
    pub topic_id: TopicId,
    pub content: String,
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(sender: EndpointId, content: String, timestamp: u64, topic_id: TopicId) -> Self {
        ChatMessage {
            sender,
            content,
            timestamp,
            topic_id,
        }
    }
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[{}] {}: {}\n",
            self.timestamp, self.sender, self.content
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub topic: TopicId,
    pub endpoints: Vec<EndpointAddr>,
}

impl Ticket {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

impl Display for Ticket {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = data_encoding::BASE32_NOPAD.decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}

pub struct ChatClient {
    id: EndpointId,
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Router,
    gossip_sender: HashMap<TopicId, GossipSender>,
    gossip_receiver: HashMap<TopicId, GossipReceiver>,
}

impl ChatClient {
    pub async fn new(path_buf: PathBuf) -> anyhow::Result<Self> {
        let secret = load_secret_key(path_buf.join("secret.key")).await?;

        let endpoint = Endpoint::builder().secret_key(secret).bind().await?;

        let gossip = Gossip::builder()
            .max_message_size(1_048_576)
            .spawn(endpoint.clone());

        let router = Router::builder(endpoint.clone())
            .accept(ALPN, gossip.clone())
            .spawn();

        Ok(ChatClient {
            id: endpoint.id(),
            endpoint,
            gossip,
            _router: router,
            gossip_sender: HashMap::new(),
            gossip_receiver: HashMap::new(),
        })
    }

    pub fn listen(&mut self, topic_id: &TopicId) -> anyhow::Result<UnboundedReceiver<Message>> {
        let mut receiver = self
            .gossip_receiver
            .remove(topic_id)
            .ok_or_else(|| anyhow::anyhow!("No gossip receiver for topic"))?;

        let (tx, rx) = unbounded_channel::<Message>();

        tokio::spawn(async move {
            loop {
                let event_option = receiver.next().await;
                match event_option {
                    Some(Ok(Event::Received(msg))) => {
                        if let Ok(chat_message) =
                            serde_json::from_slice::<ChatMessage>(&msg.content)
                        {
                            tx.send(Message::Chat(chat_message))
                                .expect("Failed to send message");
                        } else if let Ok(update_topic_message) =
                            serde_json::from_slice::<TopicMetadataMessage>(&msg.content)
                        {
                            tx.send(Message::TopicMetadata(update_topic_message))
                                .expect("Failed to send update topic message");
                        } else if let Ok(join_message) =
                            serde_json::from_slice::<JoinMessage>(&msg.content)
                        {
                            tx.send(Message::JoinTopic(join_message))
                                .expect("Failed to send join message");
                        }
                    }
                    Some(Ok(Event::NeighborUp(_))) => continue,
                    Some(Ok(Event::NeighborDown(_))) => continue,
                    Some(Ok(Event::Lagged)) => continue,
                    Some(Err(_)) => continue,
                    None => break,
                }
            }
        });

        Ok(rx)
    }

    async fn subscribe(
        &mut self,
        topic_id: TopicId,
        bootstrap: Vec<EndpointAddr>,
    ) -> anyhow::Result<()> {
        sleep(Duration::from_millis(100)).await;
        let endpoint_ids: Vec<EndpointId> = bootstrap.iter().map(|addr| addr.id).collect();

        let (sender, receiver) = self.gossip.subscribe(topic_id, endpoint_ids).await?.split();

        self.gossip_sender.insert(topic_id, sender);
        self.gossip_receiver.insert(topic_id, receiver);

        Ok(())
    }

    pub async fn send(&mut self, message: Message) -> anyhow::Result<()> {
        match message {
            Message::Chat(chat_message) => {
                self.send_message(chat_message).await?;
            }
            Message::TopicMetadata(update_topic_message) => {
                self.send_topic_metadata(update_topic_message).await?;
            }
            Message::JoinTopic(topic_id) => {
                self.send_join_topic(topic_id).await?;
            }
            Message::LeaveTopic => {}
        }
        Ok(())
    }

    async fn send_message(&mut self, message: ChatMessage) -> anyhow::Result<()> {
        let sender = self
            .gossip_sender
            .get_mut(&message.topic_id)
            .ok_or_else(|| anyhow::anyhow!("Not subscribed to topic"))?;

        let serialized = serde_json::to_vec(&message)?;
        sender.broadcast(serialized.into()).await?;

        Ok(())
    }

    async fn send_topic_metadata(&mut self, message: TopicMetadataMessage) -> anyhow::Result<()> {
        let sender = self
            .gossip_sender
            .get_mut(&message.topic)
            .ok_or_else(|| anyhow::anyhow!("Not subscribed to topic"))?;

        let serialized = serde_json::to_vec(&message)?;
        sender.broadcast(serialized.into()).await?;
        Ok(())
    }

    async fn send_join_topic(&mut self, message: JoinMessage) -> anyhow::Result<()> {
        let sender = self
            .gossip_sender
            .get_mut(&message.topic)
            .ok_or_else(|| anyhow::anyhow!("Not subscribed to topic"))?;

        let serialized = serde_json::to_vec(&message)?;
        sender.broadcast(serialized.into()).await?;
        Ok(())
    }

    pub fn peer_id(&self) -> &EndpointId {
        &self.id
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub async fn endpoint_addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    pub async fn create_topic(&mut self) -> anyhow::Result<Ticket> {
        let topic_id = TopicId::from_bytes(rand::random());

        self.subscribe(topic_id, vec![]).await?;

        let ticket = Ticket {
            topic: topic_id,
            endpoints: vec![self.endpoint.addr()],
        };

        Ok(ticket)
    }

    pub async fn join_topic(&mut self, ticket: Ticket) -> anyhow::Result<TopicId> {
        let topic_id = ticket.topic;
        let endpoints = ticket.endpoints;

        self.subscribe(topic_id, endpoints).await?;

        Ok(topic_id)
    }

    pub async fn join_topic_from_string(&mut self, ticket_str: &str) -> anyhow::Result<TopicId> {
        let ticket = Ticket::from_str(ticket_str)?;
        self.join_topic(ticket).await
    }

    pub async fn leave_topic(&mut self, topic_id: &TopicId) -> anyhow::Result<()> {
        self.gossip_sender.remove(topic_id);
        self.gossip_receiver.remove(topic_id);
        Ok(())
    }
}

pub async fn load_secret_key(path_buf: PathBuf) -> anyhow::Result<SecretKey> {
    if path_buf.exists() {
        let secret_key_bytes = tokio::fs::read(path_buf).await?;
        let secret_key = SecretKey::try_from(&secret_key_bytes[0..32])?;
        Ok(secret_key)
    } else {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let secret_key_bytes = secret_key.to_bytes();
        if let Some(parent) = path_buf.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path_buf, &secret_key_bytes).await?;
        Ok(secret_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    #[serial]
    async fn test_chat_message_serialization() {
        let endpoint = Endpoint::builder()
            .secret_key(SecretKey::generate(&mut rand::rng()))
            .bind()
            .await
            .expect("Failed to create endpoint");

        let original_message = ChatMessage::new(
            endpoint.id(),
            "Hello, world!".to_string(),
            1625247600000,
            TopicId::from_bytes(rand::random()),
        );

        let serialized =
            serde_json::to_vec(&original_message).expect("Failed to serialize chat message");
        let deserialized: ChatMessage =
            serde_json::from_slice(&serialized).expect("Failed to deserialize chat message");

        assert_eq!(original_message.sender, deserialized.sender);
        assert_eq!(original_message.content, deserialized.content);
        assert_eq!(original_message.timestamp, deserialized.timestamp);
    }

    #[tokio::test]
    #[serial]
    async fn test_chat_client_creation() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");
        assert!(client.gossip_sender.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_subscribe_to_topic() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");
        let ticket = client.create_topic().await.expect("Failed to create topic");

        assert!(client.gossip_sender.contains_key(&ticket.topic));
    }

    #[tokio::test]
    #[serial]
    async fn test_send_and_receive_message() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let mut client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        let ticket = client1
            .create_topic()
            .await
            .expect("Failed to create topic");

        client2
            .join_topic(ticket.clone())
            .await
            .expect("Failed to join topic");

        sleep(Duration::from_secs(2)).await;

        let client1_id = *client1.peer_id();
        let client2_id = *client2.peer_id();

        sleep(Duration::from_secs(2)).await;

        let message1 = "Hello from client1";
        let timestamp1 = 1625247600000;
        client1
            .send(Message::Chat(ChatMessage::new(
                client1_id,
                message1.to_string(),
                timestamp1,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client1");

        sleep(Duration::from_secs(1)).await;

        let message2 = "Hello from client2";
        let timestamp2 = 1625247600001;
        client2
            .send(Message::Chat(ChatMessage::new(
                client2_id,
                message2.to_string(),
                timestamp2,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client2");

        let receiver1 = client1
            .gossip_receiver
            .get_mut(&ticket.topic)
            .expect("Failed to get gossip receiver for client1");

        let receiver2 = client2
            .gossip_receiver
            .get_mut(&ticket.topic)
            .expect("Failed to get gossip receiver for client2");

        let mut messages_received_by_client1 = Vec::new();
        let mut messages_received_by_client2 = Vec::new();

        tokio::select! {
            _ = async {
                let collection_duration = Duration::from_secs(5);
                let start = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        result = receiver1.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result
                                && let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client1.push(chat_message);
                                }
                        }
                        result = receiver2.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result
                                && let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client2.push(chat_message);

                            }
                        }
                        _ = sleep(Duration::from_millis(100)) => {
                            if start.elapsed() >= collection_duration {
                                break;
                            }
                        }
                    }
                }
            } => {}
            _ = sleep(Duration::from_secs(20)) => {
                panic!("Test timed out");
            }
        }

        assert!(
            messages_received_by_client2.iter().any(|m| {
                m.sender == client1_id && m.content == message1 && m.timestamp == timestamp1
            }),
            "Client2 should have received message from client1. Received {} messages",
            messages_received_by_client2.len()
        );

        assert!(
            messages_received_by_client1.iter().any(|m| {
                m.sender == client2_id && m.content == message2 && m.timestamp == timestamp2
            }),
            "Client1 should have received message from client2. Received {} messages",
            messages_received_by_client1.len()
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_send_and_receive_message_three_clients() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir3 = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let mut client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");
        let mut client3 = ChatClient::new(temp_dir3.path().to_path_buf())
            .await
            .expect("Failed to create client3");

        let ticket = client1
            .create_topic()
            .await
            .expect("Failed to create topic");

        client2
            .join_topic(ticket.clone())
            .await
            .expect("Failed to join topic for client2");

        client3
            .join_topic(ticket.clone())
            .await
            .expect("Failed to join topic for client3");

        sleep(Duration::from_secs(3)).await;

        let client1_id = *client1.peer_id();
        let client2_id = *client2.peer_id();
        let client3_id = *client3.peer_id();

        let message1 = "Hello from client1";
        let timestamp1 = 1625247600000;
        client1
            .send(Message::Chat(ChatMessage::new(
                client1_id,
                message1.to_string(),
                timestamp1,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client1");

        let message2 = "Hello from client2";
        let timestamp2 = 1625247600001;
        client2
            .send(Message::Chat(ChatMessage::new(
                client2_id,
                message2.to_string(),
                timestamp2,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client2");

        let message3 = "Hello from client3";
        let timestamp3 = 1625247600002;
        client3
            .send(Message::Chat(ChatMessage::new(
                client3_id,
                message3.to_string(),
                timestamp3,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client3");

        let receiver1 = client1
            .gossip_receiver
            .get_mut(&ticket.topic)
            .expect("Failed to get gossip receiver for client1");
        let receiver2 = client2
            .gossip_receiver
            .get_mut(&ticket.topic)
            .expect("Failed to get gossip receiver for client2");
        let receiver3 = client3
            .gossip_receiver
            .get_mut(&ticket.topic)
            .expect("Failed to get gossip receiver for client3");

        let mut messages_received_by_client1 = Vec::new();
        let mut messages_received_by_client2 = Vec::new();
        let mut messages_received_by_client3 = Vec::new();

        tokio::select! {
            _ = async {
                let collection_duration = Duration::from_secs(5);
                let start = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        result = receiver1.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result
                                && let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client1.push(chat_message);
                                }
                        }
                        result = receiver2.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result
                                && let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client2.push(chat_message);
                                }
                        }
                        result = receiver3.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result
                                && let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client3.push(chat_message);
                                }
                        }
                        _ = sleep(Duration::from_millis(100)) => {
                            if start.elapsed() >= collection_duration {
                                break;
                            }
                        }
                    }
                }
            } => {}
            _ = sleep(Duration::from_secs(20)) => {
                panic!("Test timed out");
            }
        }

        assert!(
            messages_received_by_client2
                .iter()
                .any(|m| m.sender == client1_id && m.content == message1),
            "Client2 should have received message from client1"
        );
        assert!(
            messages_received_by_client3
                .iter()
                .any(|m| m.sender == client1_id && m.content == message1),
            "Client3 should have received message from client1"
        );

        assert!(
            messages_received_by_client1
                .iter()
                .any(|m| m.sender == client2_id && m.content == message2),
            "Client1 should have received message from client2"
        );
        assert!(
            messages_received_by_client3
                .iter()
                .any(|m| m.sender == client2_id && m.content == message2),
            "Client3 should have received message from client2"
        );

        assert!(
            messages_received_by_client1
                .iter()
                .any(|m| m.sender == client3_id && m.content == message3),
            "Client1 should have received message from client3"
        );
        assert!(
            messages_received_by_client2
                .iter()
                .any(|m| m.sender == client3_id && m.content == message3),
            "Client2 should have received message from client3"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_peer_id() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");
        let peer_id = *client.peer_id();
        assert_eq!(peer_id, client.id);
    }
}
