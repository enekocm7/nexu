use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageTypes {
    Chat(ChatMessage),
    JoinTopic(JoinMessage),
    LeaveTopic(LeaveMessage),
    DisconnectTopic(DisconnectMessage),
    TopicMetadata(TopicMetadataMessage),
    TopicMessages(TopicMessagesMessage),
}

pub trait GossipMessage: Serialize {
    fn topic_id(&self) -> &TopicId;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMessagesMessage {
    pub topic: TopicId,
    pub messages: Vec<ChatMessage>,
}

impl TopicMessagesMessage {
    pub fn new(topic: TopicId, messages: Vec<ChatMessage>) -> Self {
        TopicMessagesMessage { topic, messages }
    }

    pub fn new_empty(topic: TopicId) -> Self {
        TopicMessagesMessage {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisconnectMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl DisconnectMessage {
    pub fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        DisconnectMessage {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaveMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl LeaveMessage {
    pub fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        LeaveMessage {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinMessage {
    pub topic: TopicId,
    pub endpoint: EndpointId,
    pub timestamp: u64,
}

impl JoinMessage {
    pub fn new(topic: TopicId, endpoint: EndpointId, timestamp: u64) -> Self {
        JoinMessage {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMetadataMessage {
    pub topic: TopicId,
    pub name: String,
    pub avatar_url: Option<String>,
    pub timestamp: u64,
    pub members: Vec<String>,
}

impl TopicMetadataMessage {
    pub fn new(topic: TopicId, name: &str, avatar_url: Option<String>, timestamp: u64, members: Vec<String>) -> Self {
        TopicMetadataMessage {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DmMessageTypes {
    Chat(DmChatMessage),
    ProfileMetadata(DmProfileMetadataMessage),
    JoinPetition(DmJoinMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmProfileMetadataMessage {
    pub id: EndpointId,
    pub username: String,
    pub avatar_url: Option<String>,
    pub last_connection: u64,
}

impl DmProfileMetadataMessage {
    pub fn new(
        id: EndpointId,
        username: String,
        avatar_url: Option<String>,
        last_connection: u64,
    ) -> Self {
        DmProfileMetadataMessage {
            id,
            username,
            avatar_url,
            last_connection,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmChatMessage {
    pub sender: EndpointId,
    pub receiver: EndpointId,
    pub content: String,
    pub timestamp: u64,
}

impl DmChatMessage {
    pub fn new(sender: EndpointId, receiver: EndpointId, content: String, timestamp: u64) -> Self {
        DmChatMessage {
            sender,
            receiver,
            content,
            timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmJoinMessage {
    pub petitioner: EndpointId,
    pub target: EndpointId,
    pub timestamp: u64,
}

impl DmJoinMessage {
    pub fn new(
        petitioner: EndpointId,
        target: EndpointId,
        timestamp: u64,
    ) -> Self {
        DmJoinMessage {
            petitioner,
            target,
            timestamp,
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
            1625247600000,
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
