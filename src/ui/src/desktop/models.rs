use std::collections::HashMap;
use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_connection: Option<u64>,
    pub last_message: Option<String>,
    pub messages: Vec<Message>,
}

impl Topic {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            avatar_url: None,
            last_connection: None,
            last_message: None,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.last_message = Some(message.content.clone());
        self.messages.push(message);
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

impl AppState {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
            current_topic_id: None,
        }
    }

    pub fn add_topic(&mut self, topic: Topic) {
        self.topics.insert(topic.id.clone(), topic);
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

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct Message {
    pub sender_id: String,
    pub topic_id: String,
    pub content: String,
    pub timestamp: u64,
    pub is_sent: bool,
}

impl Message {
    pub fn new(sender_id: String, topic_id: String, content: String, timestamp: u64, is_sent: bool) -> Self {
        Self {
            sender_id,
            topic_id,
            content,
            timestamp,
            is_sent,
        }
    }
}