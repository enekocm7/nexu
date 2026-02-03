use iroh::EndpointId;
use iroh_blobs::Hash;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

/// Enum representing the different types of messages that can be sent over the gossip network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageTypes {
    /// A regular chat message sent to a topic.
    Chat(ChatMessage),
    /// A notification that a peer has joined a topic.
    JoinTopic(JoinMessage),
    /// A notification that a peer has left a topic.
    LeaveTopic(LeaveMessage),
    /// A notification that a peer has disconnected from a topic.
    DisconnectTopic(DisconnectMessage),
    /// Metadata about a topic (name, avatar, members).
    TopicMetadata(TopicMetadataMessage),
    /// A batch of messages for a topic, useful for syncing history.
    TopicMessages(TopicMessagesMessage),
    /// A notification about a blob (file/image) shared in the topic.
    Blob(BlobMessage),
}

/// A trait for messages that are associated with a specific gossip topic.
pub trait GossipMessage: Serialize {
    /// Returns the topic ID associated with this message.
    fn topic_id(&self) -> &TopicId;
}

/// Represents a file or binary object shared in a chat.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlobMessage {
    /// The topic where this blob was shared.
    pub topic: TopicId,
    /// The ID of the sender.
    pub sender: EndpointId,
    /// The name of the file/blob.
    pub name: String,
    /// The size of the blob in bytes.
    pub size: u64,
    /// The hash of the blob, used to retrieve it from the iroh-blobs store.
    pub hash: Hash,
    /// The timestamp when the blob was shared.
    pub timestamp: u64,
    /// The type of blob (Image, File, etc.).
    pub blob_type: BlobType,
}

impl BlobMessage {
    #[must_use]
    pub const fn new(
        topic: TopicId,
        sender: EndpointId,
        name: String,
        size: u64,
        hash: Hash,
        timestamp: u64,
        blob_type: BlobType,
    ) -> Self {
        Self {
            topic,
            sender,
            name,
            size,
            hash,
            timestamp,
            blob_type,
        }
    }
}

/// Categorizes the type of content in a [`BlobMessage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobType {
    Image,
    BigImage,
    File,
    Audio,
    Video,
    Other,
}

impl GossipMessage for BlobMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// Represents a collection of chat messages for a specific topic.
/// Often used for syncing history or sending batched updates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMessagesMessage {
    pub topic: TopicId,
    pub messages: Vec<ChatMessage>,
}

impl TopicMessagesMessage {
    #[must_use]
    pub const fn new(topic: TopicId, messages: Vec<ChatMessage>) -> Self {
        Self { topic, messages }
    }

    #[must_use]
    pub const fn new_empty(topic: TopicId) -> Self {
        Self {
            topic,
            messages: Vec::new(),
        }
    }
}

impl GossipMessage for TopicMessagesMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// A message indicating a peer has disconnected unexpectedly or explicitly from a topic context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisconnectMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl DisconnectMessage {
    #[must_use]
    pub const fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        Self {
            topic,
            endpoint,
            timestamp,
        }
    }
}

impl GossipMessage for DisconnectMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// A message indicating a peer has intentionally left a topic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaveMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl LeaveMessage {
    #[must_use]
    pub const fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        Self {
            topic,
            endpoint,
            timestamp,
        }
    }
}

impl GossipMessage for LeaveMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// A message indicating a peer has joined a topic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl JoinMessage {
    #[must_use]
    pub const fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        Self {
            topic,
            endpoint,
            timestamp,
        }
    }
}

impl GossipMessage for JoinMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// Contains metadata describing a topic, such as its display name and members.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMetadataMessage {
    pub topic: TopicId,
    pub name: String,
    pub avatar_url: Option<String>,
    pub timestamp: u64,
    /// A list of member identifiers (as strings) currently in the topic.
    pub members: Vec<String>,
}

impl TopicMetadataMessage {
    #[must_use]
    pub fn new(
        topic: TopicId,
        name: &str,
        avatar_url: Option<String>,
        timestamp: u64,
        members: Vec<String>,
    ) -> Self {
        Self {
            topic,
            name: name.to_string(),
            avatar_url,
            timestamp,
            members,
        }
    }
}

impl GossipMessage for TopicMetadataMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic
    }
}

/// A standard text-based chat message sent to a topic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: EndpointId,
    pub topic_id: TopicId,
    pub content: String,
    pub timestamp: u64,
}

impl ChatMessage {
    #[must_use]
    pub const fn new(
        sender: EndpointId,
        content: String,
        timestamp: u64,
        topic_id: TopicId,
    ) -> Self {
        Self {
            sender,
            topic_id,
            content,
            timestamp,
        }
    }
}

impl GossipMessage for ChatMessage {
    fn topic_id(&self) -> &TopicId {
        &self.topic_id
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

/// Enum representing types of messages sent via Direct Message (DM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DmMessageTypes {
    Chat(DmChatMessage),
    ProfileMetadata(DmProfileMetadataMessage),
    JoinPetition(DmJoinMessage),
    Blob(DmBlobMessage),
}

/// Carries profile information for a user in a direct message context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmProfileMetadataMessage {
    pub id: EndpointId,
    pub username: String,
    pub avatar_url: Option<String>,
    pub last_connection: u64,
}

impl DmProfileMetadataMessage {
    #[must_use]
    pub const fn new(
        id: EndpointId,
        username: String,
        avatar_url: Option<String>,
        last_connection: u64,
    ) -> Self {
        Self {
            id,
            username,
            avatar_url,
            last_connection,
        }
    }
}

/// A direct chat message between two peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmChatMessage {
    pub sender: EndpointId,
    pub receiver: EndpointId,
    pub content: String,
    pub timestamp: u64,
}

impl DmChatMessage {
    #[must_use]
    pub const fn new(sender: EndpointId, receiver: EndpointId, content: String, timestamp: u64) -> Self {
        Self {
            sender,
            receiver,
            content,
            timestamp,
        }
    }
}

/// A request from one peer to another to join a resource or group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmJoinMessage {
    pub petitioner: EndpointId,
    pub target: EndpointId,
    pub timestamp: u64,
}

impl DmJoinMessage {
    #[must_use]
    pub const fn new(petitioner: EndpointId, target: EndpointId, timestamp: u64) -> Self {
        Self {
            petitioner,
            target,
            timestamp,
        }
    }
}

/// A file or binary object shared directly between two peers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DmBlobMessage {
    pub sender: EndpointId,
    pub receiver: EndpointId,
    pub name: String,
    pub size: u64,
    pub hash: Hash,
    pub timestamp: u64,
    pub blob_type: BlobType,
}

impl DmBlobMessage {
    #[must_use]
    pub const fn new(
        sender: EndpointId,
        receiver: EndpointId,
        name: String,
        size: u64,
        hash: Hash,
        timestamp: u64,
        blob_type: BlobType,
    ) -> Self {
        Self {
            sender,
            receiver,
            name,
            size,
            hash,
            timestamp,
            blob_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::{Endpoint, SecretKey};
    use serial_test::serial;

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
            1_625_247_600_000,
            TopicId::from_bytes(rand::random()),
        );

        let serialized =
            postcard::to_allocvec(&original_message).expect("Failed to serialize chat message");
        let deserialized: ChatMessage =
            postcard::from_bytes(&serialized).expect("Failed to deserialize chat message");

        assert_eq!(original_message.sender, deserialized.sender);
        assert_eq!(original_message.content, deserialized.content);
        assert_eq!(original_message.timestamp, deserialized.timestamp);
    }
}
