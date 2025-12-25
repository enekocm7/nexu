use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_connection: Option<u64>,
    pub last_message: Option<String>,
    pub messages: Vec<Message>,
    pub last_changed: u64,
}

impl Topic {
    pub fn new(id: String, name: String, avatar_url: Option<String>) -> Self {
        Self {
            id,
            name,
            avatar_url,
            last_connection: None,
            last_message: None,
            messages: Vec::new(),
            last_changed: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn new_placeholder(id: String) -> Self {
        Self {
            id: id.clone(),
            name: id,
            avatar_url: None,
            last_connection: None,
            last_message: None,
            messages: Vec::new(),
            last_changed: 0,
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.last_message = Some(message.content.clone());
        self.messages.push(Message::Chat(message));
        self.messages.sort()
    }

    pub fn add_leave_message(&mut self, message: LeaveMessage) {
        self.messages.push(Message::Leave(message));
    }

    pub fn add_join_message(&mut self, message: JoinMessage) {
        self.messages.push(Message::Join(message));
    }

    pub fn add_disconnect_message(&mut self, message: DisconnectMessage) {
        self.messages.push(Message::Disconnect(message));
    }
}

impl PartialEq for Topic {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum TopicCreationMode {
    Create,
    Join,
}

#[cfg(feature = "desktop-web")]
#[derive(Debug, Clone)]
pub struct AppState {
    topics: HashMap<String, Topic>,
    current_topic_id: Option<String>,
}

#[cfg(feature = "desktop-web")]
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "desktop-web")]
impl AppState {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
            current_topic_id: None,
        }
    }

    pub fn add_topic(&mut self, topic: &Topic) {
        self.topics.insert(topic.id.clone(), topic.clone());
    }

    pub fn modify_topic_name(&mut self, topic_id: &str, new_name: &str) {
        if let Some(topic) = self.topics.get_mut(topic_id) {
            topic.name = new_name.to_string();
        }
    }

    pub fn modify_topic_avatar(&mut self, topic_id: &str, avatar_url: Option<String>) {
        if let Some(topic) = self.topics.get_mut(topic_id) {
            topic.avatar_url = avatar_url;
        }
    }

    pub fn set_last_changed_to_now(&mut self, topic_id: &str) -> u64 {
        if let Some(topic) = self.topics.get_mut(topic_id) {
            let now = chrono::Utc::now().timestamp_millis() as u64;
            topic.last_changed = now;
            return now;
        }
        0
    }

    pub fn set_last_changed(&mut self, topic_id: &str, timestamp: u64) {
        if let Some(topic) = self.topics.get_mut(topic_id) {
            topic.last_changed = timestamp;
        }
    }

    pub fn remove_topic(&mut self, topic_id: &str) {
        self.topics.remove(topic_id);
        if let Some(current_id) = &self.current_topic_id
            && current_id == topic_id
        {
            self.current_topic_id = None;
        }
    }

    pub fn set_current_topic(&mut self, topic_id: String) {
        self.current_topic_id = Some(topic_id);
    }

    pub fn get_current_topic(&self) -> Option<&Topic> {
        match &self.current_topic_id {
            Some(id) => self.topics.get(id),
            None => None,
        }
    }

    pub fn get_topic(&mut self, topic_id: &str) -> Option<&mut Topic> {
        self.topics.get_mut(topic_id)
    }

    pub fn get_topic_immutable(&self, topic_id: &str) -> Option<&Topic> {
        self.topics.get(topic_id)
    }

    pub fn get_all_topics(&self) -> Vec<Topic> {
        self.topics.values().cloned().collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    Chat(ChatMessage),
    Leave(LeaveMessage),
    Join(JoinMessage),
    Disconnect(DisconnectMessage),
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<Self> for Message {
    fn eq(&self, other: &Self) -> bool {
        let self_timestamp = match self {
            Message::Chat(msg) => msg.timestamp,
            Message::Leave(msg) => msg.timestamp,
            Message::Join(msg) => msg.timestamp,
            Message::Disconnect(msg) => msg.timestamp,
        };

        let other_timestamp = match other {
            Message::Chat(msg) => msg.timestamp,
            Message::Leave(msg) => msg.timestamp,
            Message::Join(msg) => msg.timestamp,
            Message::Disconnect(msg) => msg.timestamp,
        };

        self_timestamp == other_timestamp
    }
}

impl Eq for Message {}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_timestamp = match self {
            Message::Chat(msg) => msg.timestamp,
            Message::Leave(msg) => msg.timestamp,
            Message::Join(msg) => msg.timestamp,
            Message::Disconnect(msg) => msg.timestamp,
        };

        let other_timestamp = match other {
            Message::Chat(msg) => msg.timestamp,
            Message::Leave(msg) => msg.timestamp,
            Message::Join(msg) => msg.timestamp,
            Message::Disconnect(msg) => msg.timestamp,
        };

        self_timestamp.cmp(&other_timestamp)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender_id: String,
    pub topic_id: String,
    pub content: String,
    pub timestamp: u64,
    pub is_sent: bool,
}

impl ChatMessage {
    pub fn new(
        sender_id: String,
        topic_id: String,
        content: String,
        timestamp: u64,
        is_sent: bool,
    ) -> Self {
        Self {
            sender_id,
            topic_id,
            content,
            timestamp,
            is_sent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaveMessage {
    pub sender_id: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JoinMessage {
    pub sender_id: String,
    pub me: bool,
    pub timestamp: u64,
}

impl JoinMessage {
    pub fn new(sender_id: String, timestamp: u64) -> Self {
        Self {
            sender_id,
            me: false,
            timestamp,
        }
    }

    pub fn new_me(timestamp: u64) -> Self {
        Self {
            sender_id: "You".to_string(),
            me: true,
            timestamp,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisconnectMessage {
    pub sender_id: String,
    pub timestamp: u64,
}
