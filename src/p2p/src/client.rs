use crate::messages::{DmMessageTypes, GossipMessage, MessageTypes};
use crate::protocol::{write_frame, DMProtocol, DM_ALPN};
use crate::types::Ticket;
use crate::utils::load_secret_key;
use flume::Receiver;
use futures_lite::StreamExt;
use iroh::discovery::dns::DnsDiscovery;
use iroh::discovery::pkarr::PkarrPublisher;
use iroh::endpoint::SendStream;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode};
use iroh_blobs::api::blobs::{AddProgress, BlobStatus, ExportProgress};
use iroh_blobs::api::downloader::{DownloadProgress, Downloader};
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobsProtocol, Hash};
use iroh_gossip::api::{Event, GossipReceiver, GossipSender};
use iroh_gossip::{net::Gossip, proto::TopicId, ALPN};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

pub struct ChatClient {
    endpoint: Endpoint,
    gossip: Gossip,
    _router: Router,
    gossip_sender: HashMap<TopicId, GossipSender>,
    gossip_receiver: HashMap<TopicId, GossipReceiver>,
    dm_sender: HashMap<EndpointId, SendStream>,
    listen_tasks: HashMap<TopicId, tokio::task::JoinHandle<()>>,
    dm_incoming: Receiver<(EndpointId, DmMessageTypes)>,
    store: FsStore,
    temp_store_path: PathBuf,
    downloader: Downloader,
}

impl ChatClient {
    pub async fn new(path_buf: PathBuf) -> anyhow::Result<Self> {
        let secret = load_secret_key(path_buf.join("key")).await?;

        let endpoint = Endpoint::empty_builder(RelayMode::Default)
            .secret_key(secret)
            .discovery(PkarrPublisher::n0_dns())
            .discovery(DnsDiscovery::n0_dns())
            .bind()
            .await?;

        let gossip = Gossip::builder()
            .max_message_size(1_048_576)
            .spawn(endpoint.clone());

        let (dm_tx, dm_rx) = flume::unbounded();
        let dm_protocol = DMProtocol { tx: dm_tx };

        let store = FsStore::load(path_buf.join("store")).await?;
        let temp_store_path = path_buf.join("temp");

        let blobs = BlobsProtocol::new(&store, None);

        let router = Router::builder(endpoint.clone())
            .accept(ALPN, gossip.clone())
            .accept(DM_ALPN, dm_protocol.clone())
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();

        Ok(ChatClient {
            endpoint: endpoint.clone(),
            gossip,
            _router: router,
            gossip_sender: HashMap::new(),
            gossip_receiver: HashMap::new(),
            dm_sender: HashMap::new(),
            listen_tasks: HashMap::new(),
            dm_incoming: dm_rx,
            store: store.clone(),
            temp_store_path,
            downloader: store.downloader(&endpoint),
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
            MessageTypes::Blob(msg) => msg.topic_id(),
        };

        let sender = self
            .gossip_sender
            .get_mut(topic_id)
            .ok_or_else(|| anyhow::anyhow!("Not subscribed to topic"))?;

        let serialized = postcard::to_stdvec(&message)?;
        sender.broadcast(serialized.into()).await?;
        Ok(())
    }

    pub fn save_blob(&mut self, data: &[u8]) -> AddProgress<'_> {
        self.store.blobs().add_slice(data)
    }

    pub fn save_blob_from_path<P: AsRef<Path>>(&mut self, path: P) -> AddProgress<'_> {
        self.store.blobs().add_path(path)
    }

    pub fn download_blob(&mut self, blob_ticket: &BlobTicket) -> DownloadProgress {
        let downloader = self.downloader.clone();
        downloader.download(blob_ticket.hash(), Some(blob_ticket.addr().id))
    }

    pub async fn get_blob_path(&self, hash: impl Into<Hash>) -> anyhow::Result<PathBuf> {
        let hash: Hash = hash.into();
        let mut path = self.temp_store_path.join(hash.to_string());
        //path.add_extension("jpeg");
        self.store.blobs().export(hash, path.clone()).await?;
        Ok(path.clone())
    }

    pub async fn save_blob_to_storage(
        &self,
        hash: impl Into<Hash>,
        path: PathBuf,
    ) -> ExportProgress {
        let hash: Hash = hash.into();
        self.store.blobs().export(hash, path)
    }

    pub async fn has_blob(&self, hash: impl Into<Hash>) -> anyhow::Result<bool> {
        if let BlobStatus::Complete{ .. } = self.store().status(hash).await? {
            Ok(true)
        } else {
            Ok(false)
        }

    }

    pub fn peer_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn endpoint_addr(&self) -> EndpointAddr {
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
        let ticket = FromStr::from_str(ticket_str)?;
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

    pub async fn connect_peer(&mut self, addr: impl Into<EndpointAddr>) -> anyhow::Result<()> {
        let addr: EndpointAddr = addr.into();

        if self.dm_sender.contains_key(&addr.id) {
            return Ok(());
        }

        let conn = self.endpoint.connect(addr.to_owned(), DM_ALPN).await?;

        let (send, _recv) = conn.open_bi().await?;

        self.dm_sender.insert(addr.id, send);

        Ok(())
    }

    pub async fn send_dm(
        &mut self,
        addr: impl Into<EndpointAddr>,
        message: DmMessageTypes,
    ) -> anyhow::Result<()> {
        let addr: EndpointAddr = addr.into();
        let send = self
            .dm_sender
            .get_mut(&addr.id)
            .ok_or_else(|| anyhow::anyhow!("No DM sender for address"))?;

        let serialized = postcard::to_stdvec(&message)?;

        write_frame(send, &serialized).await?;

        Ok(())
    }

    pub fn incoming_dms(&self) -> Receiver<(EndpointId, DmMessageTypes)> {
        self.dm_incoming.clone()
    }

    pub fn store(&self) -> &FsStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatMessage;
    use serial_test::serial;
    use tokio::time::{sleep, Duration};

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

    #[tokio::test]
    #[serial]
    async fn test_dm_send_receive() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        client1
            .connect_peer(client2_id)
            .await
            .expect("Failed to connect");

        let msg_content =
            DmMessageTypes::ProfileMetadata(crate::messages::DmProfileMetadataMessage {
                id: client1_id,
                username: "user1".to_string(),
                avatar_url: None,
                last_connection: 12345,
            });

        client1
            .send_dm(client2_id, msg_content)
            .await
            .expect("Failed to send DM");

        let incoming = client2.incoming_dms();

        let (sender, received_msg) =
            tokio::time::timeout(Duration::from_secs(5), incoming.recv_async())
                .await
                .expect("Timeout waiting for DM")
                .expect("Failed to receive DM");

        assert_eq!(sender, client1_id);

        match received_msg {
            DmMessageTypes::ProfileMetadata(meta) => {
                assert_eq!(meta.username, "user1");
                assert_eq!(meta.last_connection, 12345);
            }
            _ => panic!("Expected ProfileMetadata"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_dm_send_without_connection_fails() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let addr2 = client2.endpoint_addr();

        let msg_content =
            DmMessageTypes::ProfileMetadata(crate::messages::DmProfileMetadataMessage {
                id: client1.peer_id(),
                username: "user1".to_string(),
                avatar_url: None,
                last_connection: 12345,
            });

        let result = client1.send_dm(addr2.clone(), msg_content).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No DM sender for address");
    }

    #[tokio::test]
    #[serial]
    async fn test_dm_bidirectional() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let mut client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        client1
            .connect_peer(client2_id)
            .await
            .expect("Failed to connect client1 to client2");
        client2
            .connect_peer(client1_id)
            .await
            .expect("Failed to connect client2 to client1");

        let msg_from_1 =
            DmMessageTypes::ProfileMetadata(crate::messages::DmProfileMetadataMessage {
                id: client1_id,
                username: "user1".to_string(),
                avatar_url: None,
                last_connection: 100,
            });

        let msg_from_2 =
            DmMessageTypes::ProfileMetadata(crate::messages::DmProfileMetadataMessage {
                id: client2_id,
                username: "user2".to_string(),
                avatar_url: None,
                last_connection: 200,
            });

        client1
            .send_dm(client2_id, msg_from_1)
            .await
            .expect("Failed to send DM from client1");

        client2
            .send_dm(client1_id, msg_from_2)
            .await
            .expect("Failed to send DM from client2");

        let incoming2 = client2.incoming_dms();
        let (sender2, received_msg2) =
            tokio::time::timeout(Duration::from_secs(5), incoming2.recv_async())
                .await
                .expect("Timeout waiting for DM on client2")
                .expect("Failed to receive DM on client2");

        assert_eq!(sender2, client1_id);
        match received_msg2 {
            DmMessageTypes::ProfileMetadata(meta) => {
                assert_eq!(meta.username, "user1");
            }
            _ => panic!("Expected ProfileMetadata"),
        }

        let incoming1 = client1.incoming_dms();
        let (sender1, received_msg1) =
            tokio::time::timeout(Duration::from_secs(5), incoming1.recv_async())
                .await
                .expect("Timeout waiting for DM on client1")
                .expect("Failed to receive DM on client1");

        assert_eq!(sender1, client2_id);
        match received_msg1 {
            DmMessageTypes::ProfileMetadata(meta) => {
                assert_eq!(meta.username, "user2");
            }
            _ => panic!("Expected ProfileMetadata"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_dm_multiple_messages() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        client1
            .connect_peer(client2_id)
            .await
            .expect("Failed to connect");

        for i in 0..5 {
            let msg = DmMessageTypes::ProfileMetadata(crate::messages::DmProfileMetadataMessage {
                id: client1_id,
                username: format!("user1_message_{}", i),
                avatar_url: None,
                last_connection: i as u64,
            });
            client1
                .send_dm(client2_id, msg)
                .await
                .expect("Failed to send DM");
        }

        let incoming = client2.incoming_dms();
        for i in 0..5 {
            let (sender, received_msg) =
                tokio::time::timeout(Duration::from_secs(5), incoming.recv_async())
                    .await
                    .expect("Timeout waiting for DM")
                    .expect("Failed to receive DM");

            assert_eq!(sender, client1_id);
            match received_msg {
                DmMessageTypes::ProfileMetadata(meta) => {
                    assert_eq!(meta.username, format!("user1_message_{}", i));
                    assert_eq!(meta.last_connection, i as u64);
                }
                _ => panic!("Expected ProfileMetadata"),
            }
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_dm_chat_message() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let client1_id = client1.peer_id();
        let client2_id = client2.peer_id();

        client1
            .connect_peer(client2_id)
            .await
            .expect("Failed to connect");

        let msg_content = DmMessageTypes::Chat(crate::messages::DmChatMessage {
            sender: client1_id,
            receiver: client2_id,
            content: "Hello DM".to_string(),
            timestamp: 123456789,
        });

        client1
            .send_dm(client2_id, msg_content)
            .await
            .expect("Failed to send DM");

        let incoming = client2.incoming_dms();

        let (sender, received_msg) =
            tokio::time::timeout(Duration::from_secs(5), incoming.recv_async())
                .await
                .expect("Timeout waiting for DM")
                .expect("Failed to receive DM");

        assert_eq!(sender, client1_id);

        match received_msg {
            DmMessageTypes::Chat(chat_msg) => {
                assert_eq!(chat_msg.sender, client1_id);
                assert_eq!(chat_msg.content, "Hello DM");
                assert_eq!(chat_msg.timestamp, 123456789);
            }
            _ => panic!("Expected Chat message"),
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_save_blob() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data = b"Hello, this is test blob data!";

        let progress = client.save_blob(test_data).await;
        let result = progress.expect("Failed to save blob");

        assert!(!result.hash.to_string().is_empty());

        let bytes = client.store().get_bytes(result.hash).await.unwrap();
        let slice = bytes.iter().as_slice();

        assert_eq!(slice, test_data);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_blob_large_data() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        let progress = client.save_blob(&test_data).await;
        let result = progress.expect("Failed to save blob");

        let bytes = client
            .store()
            .get_bytes(result.hash)
            .await
            .expect("Failed to get bytes");
        assert_eq!(bytes.as_ref(), test_data.as_slice());
    }

    #[tokio::test]
    #[serial]
    async fn test_save_blob_empty_data() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data = b"";

        let progress = client.save_blob(test_data).await;
        let result = progress.expect("Failed to save blob");

        assert!(!result.hash.to_string().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_download_blob_between_clients() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let mut client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let test_data = b"Test blob data for download test";
        let progress = client1.save_blob(test_data).await;
        let result = progress.expect("Failed to save blob");

        let blob_ticket = BlobTicket::new(
            client1.endpoint_addr(),
            result.hash,
            iroh_blobs::BlobFormat::Raw,
        );

        let download_result = client2.download_blob(&blob_ticket).await;
        download_result.expect("Failed to download blob");

        let bytes = client2
            .store()
            .get_bytes(result.hash)
            .await
            .expect("Failed to get bytes");
        assert_eq!(bytes.as_ref(), test_data);
    }

    #[tokio::test]
    #[serial]
    async fn test_download_blob_large_data_between_clients() {
        let temp_dir1 = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir2 = tempfile::tempdir().expect("Failed to create temp dir");

        let mut client1 = ChatClient::new(temp_dir1.path().to_path_buf())
            .await
            .expect("Failed to create client1");
        let mut client2 = ChatClient::new(temp_dir2.path().to_path_buf())
            .await
            .expect("Failed to create client2");

        sleep(Duration::from_secs(1)).await;

        let test_data: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        let progress = client1.save_blob(&test_data).await;
        let result = progress.expect("Failed to save blob");

        let blob_ticket = BlobTicket::new(
            client1.endpoint_addr(),
            result.hash,
            iroh_blobs::BlobFormat::Raw,
        );

        let download_result = client2.download_blob(&blob_ticket).await;
        download_result.expect("Failed to download blob");

        let bytes = client2
            .store()
            .get_bytes(result.hash)
            .await
            .expect("Failed to get bytes");
        assert_eq!(bytes.as_ref(), test_data.as_slice());
    }

    #[tokio::test]
    #[serial]
    async fn test_save_multiple_blobs() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data1 = b"First blob data";
        let test_data2 = b"Second blob data";
        let test_data3 = b"Third blob data";

        let progress1 = client.save_blob(test_data1).await;
        let result1 = progress1.expect("Failed to save blob 1");

        let progress2 = client.save_blob(test_data2).await;
        let result2 = progress2.expect("Failed to save blob 2");

        let progress3 = client.save_blob(test_data3).await;
        let result3 = progress3.expect("Failed to save blob 3");

        assert_ne!(result1.hash, result2.hash);
        assert_ne!(result2.hash, result3.hash);
        assert_ne!(result1.hash, result3.hash);
        assert_ne!(result2.hash, result3.hash);

        let bytes1 = client
            .store()
            .get_bytes(result1.hash)
            .await
            .expect("Failed to get bytes 1");
        assert_eq!(bytes1.as_ref(), test_data1);

        let bytes2 = client
            .store()
            .get_bytes(result2.hash)
            .await
            .expect("Failed to get bytes 2");
        assert_eq!(bytes2.as_ref(), test_data2);

        let bytes3 = client
            .store()
            .get_bytes(result3.hash)
            .await
            .expect("Failed to get bytes 3");
        assert_eq!(bytes3.as_ref(), test_data3);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_same_data_twice() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data = b"Duplicate blob data";

        let progress1 = client.save_blob(test_data).await;
        let result1 = progress1.expect("Failed to save blob 1");

        let progress2 = client.save_blob(test_data).await;
        let result2 = progress2.expect("Failed to save blob 2");

        assert_eq!(result1.hash, result2.hash);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_blob_from_storage() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data = b"Data for get_blob_from_storage test";
        let progress = client.save_blob(test_data).await;
        let result = progress.expect("Failed to save blob");

        let path = client
            .get_blob_path(result.hash)
            .await
            .expect("Failed to get blob from storage");

        let retrieved_bytes = std::fs::read(path).expect("Failed to read blob from path");

        assert_eq!(retrieved_bytes, test_data);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_blob_to_storage() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let mut client = ChatClient::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create chat client");

        let test_data = b"Data for save_blob_to_storage test";
        let progress = client.save_blob(test_data).await;
        let result = progress.expect("Failed to save blob");

        let expected_file_path = temp_dir.path().join(result.hash.to_string());
        let export_progress = client
            .save_blob_to_storage(result.hash, expected_file_path.clone())
            .await;

        export_progress.await.expect("Failed to export blob");

        assert!(
            expected_file_path.exists(),
            "Exported file should exist at {:?}",
            expected_file_path
        );

        let content = std::fs::read(expected_file_path).expect("Failed to read exported file");
        assert_eq!(content, test_data);
    }
}
