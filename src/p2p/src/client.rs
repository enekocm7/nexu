use crate::messages::{DmMessageTypes, GossipMessage, MessageTypes};
use crate::protocol::{DM_ALPN, DMProtocol};
use crate::ticket::Ticket;
use crate::utils::load_secret_key;
use flume::Receiver;
use futures_lite::StreamExt;
use iroh::endpoint::{RecvStream, SendStream};
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::{ALPN, net::Gossip, proto::TopicId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

pub struct ChatClient {
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Router,
    gossip_sender: HashMap<TopicId, GossipSender>,
    gossip_receiver: HashMap<TopicId, GossipReceiver>,
    dm_sender: HashMap<EndpointAddr, SendStream>,
    dm_receiver: HashMap<EndpointAddr, RecvStream>,
    listen_tasks: HashMap<TopicId, tokio::task::JoinHandle<()>>,
}

impl ChatClient {
    pub async fn new(path_buf: PathBuf) -> anyhow::Result<Self> {
        let secret = load_secret_key(path_buf.join("secret.key")).await?;

        let endpoint = Endpoint::builder().secret_key(secret).bind().await?;

        let gossip = Gossip::builder()
            .max_message_size(1_048_576)
            .spawn(endpoint.clone());

        let dm_protocol = DMProtocol;

        let router = Router::builder(endpoint.clone())
            .accept(ALPN, gossip.clone())
            .accept(DM_ALPN, dm_protocol.clone())
            .spawn();

        Ok(ChatClient {
            endpoint,
            gossip,
            _router: router,
            gossip_sender: HashMap::new(),
            gossip_receiver: HashMap::new(),
            dm_sender: HashMap::new(),
            dm_receiver: HashMap::new(),
            listen_tasks: HashMap::new(),
        })
    }

    pub fn listen(&mut self, topic_id: &TopicId) -> anyhow::Result<Receiver<MessageTypes>> {
        let mut receiver = self
            .gossip_receiver
            .remove(topic_id)
            .ok_or_else(|| anyhow::anyhow!("No gossip receiver for topic"))?;

        let (tx, rx) = flume::unbounded::<MessageTypes>();

        let handle = tokio::spawn(async move {
            loop {
                let event_option = receiver.next().await;
                match event_option {
                    Some(Ok(Event::Received(msg))) => {
                        if let Ok(message) = postcard::from_bytes::<MessageTypes>(&msg.content) {
                            tx.send(message).expect("Failed to send message");
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

        self.listen_tasks.insert(*topic_id, handle);

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

    pub async fn send(&mut self, message: MessageTypes) -> anyhow::Result<()> {
        let topic_id = match &message {
            MessageTypes::Chat(msg) => msg.topic_id(),
            MessageTypes::TopicMetadata(msg) => msg.topic_id(),
            MessageTypes::JoinTopic(msg) => msg.topic_id(),
            MessageTypes::LeaveTopic(msg) => msg.topic_id(),
            MessageTypes::DisconnectTopic(msg) => msg.topic_id(),
            MessageTypes::TopicMessages(msg) => msg.topic_id(),
        };

        let sender = self
            .gossip_sender
            .get_mut(topic_id)
            .ok_or_else(|| anyhow::anyhow!("Not subscribed to topic"))?;

        let serialized = postcard::to_stdvec(&message)?;
        sender.broadcast(serialized.into()).await?;
        Ok(())
    }

    pub fn peer_id(&self) -> EndpointId {
        self.endpoint.id()
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
        let ticket = std::str::FromStr::from_str(ticket_str)?;
        self.join_topic(ticket).await
    }

    pub async fn leave_topic(&mut self, topic_id: &TopicId) -> anyhow::Result<()> {
        self.gossip_sender.remove(topic_id);
        self.gossip_receiver.remove(topic_id);
        if let Some(handle) = self.listen_tasks.remove(topic_id) {
            handle.abort();
        }
        Ok(())
    }

    pub async fn connect_peer(&mut self, addr: &EndpointAddr) -> anyhow::Result<()> {
        let conn = self.endpoint.connect(addr.to_owned(), DM_ALPN).await?;

        let (send, recv) = conn.open_bi().await?;

        self.dm_receiver.insert(addr.to_owned(), recv);
        self.dm_sender.insert(addr.to_owned(), send);

        Ok(())
    }

    pub async fn send_dm(
        &mut self,
        addr: EndpointAddr,
        message: DmMessageTypes,
    ) -> anyhow::Result<()> {
        let send = self
            .dm_sender
            .get_mut(&addr)
            .ok_or_else(|| anyhow::anyhow!("No DM sender for address"))?;

        let serialized = postcard::to_stdvec(&message)?;

        send.write_all(&serialized).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;
    use serial_test::serial;
    use tokio::time::{Duration, sleep};

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

        let receiver1 = client1
            .listen(&ticket.topic)
            .expect("Failed to start listening on client1");
        let receiver2 = client2
            .listen(&ticket.topic)
            .expect("Failed to start listening on client2");

        sleep(Duration::from_secs(2)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        let message1 = "Hello from client1";
        let timestamp1 = 1625247600000;
        client1
            .send(MessageTypes::Chat(ChatMessage::new(
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
            .send(MessageTypes::Chat(ChatMessage::new(
                client2_id,
                message2.to_string(),
                timestamp2,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client2");

        let mut messages_received_by_client1 = Vec::new();
        let mut messages_received_by_client2 = Vec::new();

        tokio::select! {
            _ = async {
                let collection_duration = Duration::from_secs(5);
                let start = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        result = receiver1.recv_async() => {
                            if let Ok(MessageTypes::Chat(chat_message)) = result {
                                messages_received_by_client1.push(chat_message);
                            }
                        }
                        result = receiver2.recv_async() => {
                            if let Ok(MessageTypes::Chat(chat_message)) = result {
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

        let receiver1 = client1
            .listen(&ticket.topic)
            .expect("Failed to start listening on client1");
        let receiver2 = client2
            .listen(&ticket.topic)
            .expect("Failed to start listening on client2");
        let receiver3 = client3
            .listen(&ticket.topic)
            .expect("Failed to start listening on client3");

        sleep(Duration::from_secs(3)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();
        let client3_id = client3.peer_id();

        let message1 = "Hello from client1";
        let timestamp1 = 1625247600000;
        client1
            .send(MessageTypes::Chat(ChatMessage::new(
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
            .send(MessageTypes::Chat(ChatMessage::new(
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
            .send(MessageTypes::Chat(ChatMessage::new(
                client3_id,
                message3.to_string(),
                timestamp3,
                ticket.topic,
            )))
            .await
            .expect("Failed to send message from client3");

        let mut messages_received_by_client1 = Vec::new();
        let mut messages_received_by_client2 = Vec::new();
        let mut messages_received_by_client3 = Vec::new();

        tokio::select! {
            _ = async {
                let collection_duration = Duration::from_secs(5);
                let start = tokio::time::Instant::now();

                loop {
                    tokio::select! {
                        result = receiver1.recv_async() => {
                            if let Ok(MessageTypes::Chat(chat_message)) = result {
                                messages_received_by_client1.push(chat_message);
                            }
                        }
                        result = receiver2.recv_async() => {
                            if let Ok(MessageTypes::Chat(chat_message)) = result {
                                messages_received_by_client2.push(chat_message);
                            }
                        }
                        result = receiver3.recv_async() => {
                            if let Ok(MessageTypes::Chat(chat_message)) = result {
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
}
