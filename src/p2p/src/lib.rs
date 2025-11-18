use futures_lite::StreamExt;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::{net::Gossip, proto::TopicId, ALPN};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: EndpointId,
    pub content: String,
    pub timestamp: u64,
}

impl ChatMessage {
    pub fn new(sender: EndpointId, content: String, timestamp: u64) -> Self {
        ChatMessage {
            sender,
            content,
            timestamp,
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
    gossip_sender: Option<GossipSender>,
    gossip_receiver: Option<GossipReceiver>,
}

impl ChatClient {
    pub async fn new() -> anyhow::Result<Self> {
        let secret = SecretKey::generate(&mut rand::rng());

        let endpoint = Endpoint::builder().secret_key(secret).bind().await?;

        let gossip = Gossip::builder().spawn(endpoint.clone());

        let router = Router::builder(endpoint.clone())
            .accept(ALPN, gossip.clone())
            .spawn();

        Ok(ChatClient {
            id: endpoint.id(),
            endpoint,
            gossip,
            _router: router,
            gossip_sender: None,
            gossip_receiver: None,
        })
    }

    pub async fn listen(&mut self) -> anyhow::Result<()> {
        let receiver = self
            .gossip_receiver
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No gossip receiver"))?;
        loop {
            let event_option = receiver.next().await;
            match event_option {
                Some(event) => {
                    if let Event::Received(msg) = event? {
                        if let Ok(chat_message) =
                            serde_json::from_slice::<ChatMessage>(&msg.content)
                        {
                            print!("{chat_message}");
                        }
                    }
                }
                None => continue,
            }
        }
    }

    async fn subscribe(
        &mut self,
        topic_id: TopicId,
        bootstrap: Vec<EndpointAddr>,
    ) -> anyhow::Result<()> {
        sleep(Duration::from_millis(100)).await;
        let endpoint_ids: Vec<EndpointId> = bootstrap.iter().map(|addr| addr.id).collect();

        let (sender, receiver) = self.gossip.subscribe(topic_id, endpoint_ids).await?.split();

        self.gossip_sender = Some(sender);
        self.gossip_receiver = Some(receiver);

        Ok(())
    }

    pub async fn send_message(&mut self, content: String, timestamp: u64) -> anyhow::Result<()> {
        if let Some(sender) = &mut self.gossip_sender {
            let message = ChatMessage::new(self.id, content, timestamp);
            let serialized = serde_json::to_vec(&message)?;
            sender.broadcast(serialized.into()).await?;
        }
        Ok(())
    }

    pub fn peer_id(&self) -> EndpointId {
        self.id
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    #[serial]
    async fn test_chat_message_serialization() {
        let endpoint = Endpoint::builder()
            .secret_key(SecretKey::generate(&mut rand::rng()))
            .bind()
            .await
            .expect("Failed to create endpoint");

        let original_message =
            ChatMessage::new(endpoint.id(), "Hello, world!".to_string(), 1625247600000);

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
        let client = ChatClient::new()
            .await
            .expect("Failed to create chat client");
        assert!(client.gossip_sender.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_subscribe_to_topic() {
        let mut client = ChatClient::new()
            .await
            .expect("Failed to create chat client");
        client.create_topic().await.expect("Failed to create topic");

        assert!(client.gossip_sender.is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_send_and_receive_message() {
        let mut client1 = ChatClient::new().await.expect("Failed to create client1");
        let mut client2 = ChatClient::new().await.expect("Failed to create client2");

        let ticket = client1
            .create_topic()
            .await
            .expect("Failed to create topic");

        client2
            .join_topic(ticket)
            .await
            .expect("Failed to join topic");

        sleep(Duration::from_secs(2)).await;
        
        let mut receiver1 = client1
            .gossip_receiver
            .take()
            .expect("Failed to get gossip receiver for client1");
        
        let mut receiver2 = client2
            .gossip_receiver
            .take()
            .expect("Failed to get gossip receiver for client2");
        
        sleep(Duration::from_secs(2)).await;

        let message1 = "Hello from client1".to_string();
        let timestamp1 = 1625247600000;
        client1
            .send_message(message1.clone(), timestamp1)
            .await
            .expect("Failed to send message from client1");

        sleep(Duration::from_secs(1)).await;

        let message2 = "Hello from client2".to_string();
        let timestamp2 = 1625247600001;
        client2
            .send_message(message2.clone(), timestamp2)
            .await
            .expect("Failed to send message from client2");

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        let mut messages_received_by_client1 = Vec::new();
        let mut messages_received_by_client2 = Vec::new();

        tokio::select! {
            _ = async {
                let collection_duration = Duration::from_secs(5);
                let start = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        result = receiver1.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result {
                                if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client1.push(chat_message);
                                }
                            }
                        }
                        result = receiver2.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result {
                                if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client2.push(chat_message);
                                }
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
        let mut client1 = ChatClient::new().await.expect("Failed to create client1");
        let mut client2 = ChatClient::new().await.expect("Failed to create client2");
        let mut client3 = ChatClient::new().await.expect("Failed to create client3");

        let ticket = client1
            .create_topic()
            .await
            .expect("Failed to create topic");

        client2
            .join_topic(ticket.clone())
            .await
            .expect("Failed to join topic for client2");

        client3
            .join_topic(ticket)
            .await
            .expect("Failed to join topic for client3");

        let mut receiver1 = client1
            .gossip_receiver
            .take()
            .expect("Failed to get gossip receiver for client1");
        let mut receiver2 = client2
            .gossip_receiver
            .take()
            .expect("Failed to get gossip receiver for client2");
        let mut receiver3 = client3
            .gossip_receiver
            .take()
            .expect("Failed to get gossip receiver for client3");

        sleep(Duration::from_secs(3)).await;

        let message1 = "Hello from client1".to_string();
        let timestamp1 = 1625247600000;
        client1
            .send_message(message1.clone(), timestamp1)
            .await
            .expect("Failed to send message from client1");

        let message2 = "Hello from client2".to_string();
        let timestamp2 = 1625247600001;
        client2
            .send_message(message2.clone(), timestamp2)
            .await
            .expect("Failed to send message from client2");

        let message3 = "Hello from client3".to_string();
        let timestamp3 = 1625247600002;
        client3
            .send_message(message3.clone(), timestamp3)
            .await
            .expect("Failed to send message from client3");

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();
        let client3_id = client3.peer_id();

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
                            if let Ok(Some(Event::Received(msg))) = result {
                                if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client1.push(chat_message);
                                }
                            }
                        }
                        result = receiver2.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result {
                                if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client2.push(chat_message);
                                }
                            }
                        }
                        result = receiver3.try_next() => {
                            if let Ok(Some(Event::Received(msg))) = result {
                                if let Ok(chat_message) = serde_json::from_slice::<ChatMessage>(&msg.content) {
                                    messages_received_by_client3.push(chat_message);
                                }
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
        let client = ChatClient::new()
            .await
            .expect("Failed to create chat client");
        let peer_id = client.peer_id();
        assert_eq!(peer_id, client.id);
    }
}
