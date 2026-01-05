use crate::desktop::models::ConnectionStatus::Online;
use ConnectionStatus::Offline;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_connection: Option<u64>,
    pub last_message: Option<String>,
    pub messages: Vec<Message>,
    pub last_changed: u64,
    pub members: HashSet<String>,
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
            members: HashSet::new(),
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
            members: HashSet::new(),
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

    pub fn add_member(&mut self, profile_id: &str) {
        self.members.insert(profile_id.to_string());
    }

    pub fn remove_member(&mut self, profile_id: &str) {
        self.members.remove(profile_id);
    }

    pub fn get_member_ids(&self) -> Vec<&str> {
        self.members.iter().map(|s| s.as_str()).collect()
    }

    pub fn has_member(&self, profile_id: &str) -> bool {
        self.members.contains(profile_id)
    }

    pub fn update_member(&mut self, profile_id: &str) {
        self.members.replace(profile_id.to_string());
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
    contacts: HashMap<String, Profile>,
    profile: Profile,
}

#[cfg(feature = "desktop-web")]
impl AppState {
    pub fn new(profile_id: &str) -> Self {
        Self {
            topics: HashMap::new(),
            current_topic_id: None,
            contacts: HashMap::new(),
            profile: Profile::new_with_id(profile_id),
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

    pub fn get_topic_mutable(&mut self, topic_id: &str) -> Option<&mut Topic> {
        self.topics.get_mut(topic_id)
    }

    pub fn get_topic(&self, topic_id: &str) -> Option<&Topic> {
        self.topics.get(topic_id)
    }

    pub fn get_all_topics(&self) -> Vec<Topic> {
        self.topics.values().cloned().collect()
    }

    pub fn get_profile(&self) -> Profile {
        self.profile.clone()
    }

    pub fn set_profile_id(&mut self, id: &str) {
        self.profile.id = id.to_string()
    }

    pub fn set_profile_name(&mut self, name: &str) {
        self.profile.name = name.to_string()
    }

    pub fn set_profile_avatar(&mut self, avatar_url: &Option<String>) {
        self.profile.avatar = avatar_url.to_owned()
    }

    pub fn set_profile_last_connection_to_now(&mut self) {
        self.profile.last_connection = Online;
    }

    pub fn set_profile_last_connection_offline(&mut self, timestamp: u64) {
        self.profile.last_connection = Offline(timestamp);
    }

    pub fn add_contact(&mut self, profile: Profile) {
        self.contacts.insert(profile.id.clone(), profile);
    }

    pub fn get_contact(&self, profile_id: &str) -> Option<&Profile> {
        self.contacts.get(profile_id)
    }

    pub fn get_all_contacts(&self) -> Vec<Profile> {
        self.contacts.values().cloned().collect()
    }

    pub fn remove_contact(&mut self, profile_id: &str) {
        self.contacts.remove(profile_id);
    }

    pub fn modify_contact(&mut self, profile: Profile) {
        if let Some(existing_profile) = self.contacts.get_mut(&profile.id) {
            existing_profile.name = profile.name;
            existing_profile.avatar = profile.avatar;
            existing_profile.last_connection = profile.last_connection;
        }
    }

    pub fn set_contact_last_connection(&mut self, contact: &str, timestamp: u64) {
        if let Some(profile) = self.contacts.get_mut(contact) {
            profile.last_connection = Offline(timestamp);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    Chat(ChatMessage),
    Leave(LeaveMessage),
    Join(JoinMessage),
    Disconnect(DisconnectMessage),
}

impl Message {
    fn get_timestamp(&self) -> u64 {
        match self {
            Message::Chat(msg) => msg.timestamp,
            Message::Leave(msg) => msg.timestamp,
            Message::Join(msg) => msg.timestamp,
            Message::Disconnect(msg) => msg.timestamp,
        }
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<Self> for Message {
    fn eq(&self, other: &Self) -> bool {
        let self_timestamp = self.get_timestamp();

        let other_timestamp = other.get_timestamp();

        self_timestamp == other_timestamp
    }
}

impl Eq for Message {}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_timestamp = self.get_timestamp();

        let other_timestamp = other.get_timestamp();

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

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub last_connection: ConnectionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Online,
    Offline(u64),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Offline(0)
    }
}

impl PartialEq<Self> for ConnectionStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Online, Online) => true,
            (Offline(ts1), Offline(ts2)) => ts1 == ts2,
            _ => false,
        }
    }
}

impl Eq for ConnectionStatus {}

impl Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Online => "Online".to_string(),
            Offline(timestamp) => format!("Offline since {}", timestamp),
        };
        write!(f, "{}", str)
    }
}

impl Profile {
    pub fn new(id: &str, name: &str, avatar: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            avatar: Some(avatar.to_string()),
            last_connection: Online,
        }
    }

    pub fn new_with_id(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            avatar: None,
            last_connection: Online,
        }
    }
}

impl PartialEq<Self> for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Profile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
