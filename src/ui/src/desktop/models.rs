use ConnectionStatus::{Offline, Online};
use dioxus::html::FileData;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

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
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
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

    #[must_use]
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
        self.messages.sort();
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

    pub fn add_blob_message(&mut self, message: BlobMessage) {
        self.last_message = Some(format!("[{}]", message.blob_name));
        self.messages.push(Message::Blob(message));
        self.messages.sort();
    }

    pub fn add_member(&mut self, profile_id: &str) {
        self.members.insert(profile_id.to_string());
    }

    pub fn remove_member(&mut self, profile_id: &str) {
        self.members.remove(profile_id);
    }

    #[must_use]
    pub fn get_member_ids(&self) -> Vec<&str> {
        self.members.iter().map(String::as_str).collect()
    }
    
    #[must_use]
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

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum TopicCreationMode {
    Create,
    Join,
}

#[cfg(feature = "desktop")]
#[derive(Debug, Clone)]
pub struct AppState {
    topics: HashMap<String, Topic>,
    current_topic_id: Option<String>,
    contacts: HashMap<String, ProfileChat>,
    profile: Profile,
}

#[cfg(feature = "desktop")]
impl AppState {
    #[must_use]
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

    #[allow(clippy::cast_sign_loss)]
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

    #[must_use]
    pub fn get_current_topic(&self) -> Option<&Topic> {
        self.current_topic_id.as_ref().and_then(|id| self.topics.get(id))
    }

    pub fn set_topic_members(&mut self, topic_id: &str, members: Vec<String>) {
        if let Some(topic) = self.topics.get_mut(topic_id) {
            topic.members = members.into_iter().collect();
        }
    }

    #[must_use]
    pub fn get_topic_members(&self, topic_id: &str) -> Option<&HashSet<String>> {
        if let Some(topic) = self.topics.get(topic_id) {
            return Some(&topic.members);
        }
        None
    }

    pub fn get_topic_mutable(&mut self, topic_id: &str) -> Option<&mut Topic> {
        self.topics.get_mut(topic_id)
    }

    #[must_use]
    pub fn get_topic(&self, topic_id: &str) -> Option<&Topic> {
        self.topics.get(topic_id)
    }

    #[must_use]
    pub fn get_all_topics(&self) -> Vec<Topic> {
        self.topics.values().cloned().collect()
    }

    #[must_use]
    pub fn get_profile(&self) -> Profile {
        self.profile.clone()
    }

    pub fn set_profile_id(&mut self, id: &str) {
        self.profile.id = id.to_string();
    }

    pub fn set_profile_name(&mut self, name: &str) {
        self.profile.name = name.to_string();
    }

    pub fn set_profile_avatar(&mut self, avatar_url: Option<&str>) {
        self.profile.avatar = avatar_url.map(str::to_string);
    }

    pub const fn set_profile_last_connection_to_now(&mut self) {
        self.profile.last_connection = Online;
    }

    pub const fn set_profile_last_connection_offline(&mut self, timestamp: u64) {
        self.profile.last_connection = Offline(timestamp);
    }

    pub fn add_contact(&mut self, profile: Profile) {
        self.contacts
            .insert(profile.id.clone(), ProfileChat::new(profile));
    }

    pub fn add_contact_chat(&mut self, profile_chat: ProfileChat) {
        self.contacts
            .insert(profile_chat.profile.id.clone(), profile_chat);
    }

    #[must_use]
    pub fn get_contact_chat(&self, profile_id: &str) -> Option<&ProfileChat> {
        self.contacts.get(profile_id)
    }

    #[must_use]
    pub fn get_contact(&self, profile_id: &str) -> Option<&Profile> {
        self.contacts.get(profile_id).map(|prof| &prof.profile)
    }

    #[must_use]
    pub fn get_all_contacts(&self) -> Vec<Profile> {
        self.contacts
            .values()
            .cloned()
            .map(|chat| chat.profile)
            .collect()
    }

    #[must_use]
    pub fn get_all_contacts_chat(&self) -> Vec<ProfileChat> {
        self.contacts.values().cloned().collect()
    }

    pub fn remove_contact(&mut self, profile_id: &str) {
        self.contacts.remove(profile_id);
    }

    pub fn modify_contact(&mut self, profile: Profile) {
        if let Some(profile_chat) = self.contacts.get_mut(&profile.id)
            && let Some(existing_profile) = Some(&mut profile_chat.profile)
        {
            existing_profile.name = profile.name;
            existing_profile.avatar = profile.avatar;
            existing_profile.last_connection = profile.last_connection;
        }
    }

    pub fn set_contact_last_connection(&mut self, contact: &str, timestamp: u64) {
        if let Some(profile_chat) = self.contacts.get_mut(contact)
            && let Some(profile) = Some(&mut profile_chat.profile)
        {
            profile.last_connection = Offline(timestamp);
        }
    }

    pub fn add_dm_message(&mut self, profile_id: &str, message: DmChatMessage) {
        if let Some(contact) = self.contacts.get_mut(profile_id) {
            contact.add_dm_message(message);
        } else {
            let mut contact = ProfileChat::new(Profile::new_with_id(profile_id));
            contact.add_dm_message(message);
            self.contacts.insert(profile_id.to_string(), contact);
        }
    }

    pub fn add_dm_blob_message(&mut self, id: &str, message: DmBlobMessage) {
        if let Some(contact) = self.contacts.get_mut(id) {
            contact.add_dm_blob_message(message);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Message {
    Chat(ChatMessage),
    Leave(LeaveMessage),
    Join(JoinMessage),
    Disconnect(DisconnectMessage),
    Blob(BlobMessage),
}

impl Message {
    const fn get_timestamp(&self) -> u64 {
        match self {
            Self::Chat(msg) => msg.timestamp,
            Self::Leave(msg) => msg.timestamp,
            Self::Join(msg) => msg.timestamp,
            Self::Disconnect(msg) => msg.timestamp,
            Self::Blob(msg) => msg.timestamp,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender_id: String,
    pub topic_id: String,
    pub content: String,
    pub timestamp: u64,
    pub is_sent: bool,
}

impl ChatMessage {
    #[must_use]
    pub const fn new(
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobMessage {
    pub sender_id: String,
    pub topic_id: String,
    pub blob_hash: String,
    pub blob_name: String,
    pub blob_size: u64, //Size in bytes
    pub timestamp: u64,
    pub is_sent: bool,
    pub blob_type: BlobType,
}

impl BlobMessage {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        sender_id: String,
        topic_id: String,
        blob_hash: String,
        blob_name: String,
        blob_size: u64,
        timestamp: u64,
        is_sent: bool,
        blob_type: BlobType,
    ) -> Self {
        Self {
            sender_id,
            topic_id,
            blob_hash,
            blob_name,
            blob_size,
            timestamp,
            is_sent,
            blob_type,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlobType {
    Image,
    BigImage,
    File,
    Audio,
    Video,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmChatMessage {
    pub sender_id: String,
    pub receiver_id: String,
    pub content: String,
    pub timestamp: u64,
    pub is_sent: bool,
}

impl DmChatMessage {
    #[must_use]
    pub const fn new(
        sender_id: String,
        receiver_id: String,
        content: String,
        timestamp: u64,
        is_sent: bool,
    ) -> Self {
        Self {
            sender_id,
            receiver_id,
            content,
            timestamp,
            is_sent,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmBlobMessage {
    pub sender_id: String,
    pub receiver_id: String,
    pub blob_hash: String,
    pub blob_name: String,
    pub blob_size: u64,
    pub timestamp: u64,
    pub is_sent: bool,
    pub blob_type: BlobType,
}

impl DmBlobMessage {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        sender_id: String,
        receiver_id: String,
        blob_hash: String,
        blob_name: String,
        blob_size: u64,
        timestamp: u64,
        is_sent: bool,
        blob_type: BlobType,
    ) -> Self {
        Self {
            sender_id,
            receiver_id,
            blob_hash,
            blob_name,
            blob_size,
            timestamp,
            is_sent,
            blob_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmMessage {
    Chat(DmChatMessage),
    Blob(DmBlobMessage),
}

impl DmMessage {
    #[must_use]
    pub const fn get_timestamp(&self) -> u64 {
        match self {
            Self::Chat(msg) => msg.timestamp,
            Self::Blob(msg) => msg.timestamp,
        }
    }

    #[must_use]
    pub fn get_content(&self) -> String {
        match self {
            Self::Chat(msg) => msg.content.clone(),
            Self::Blob(msg) => msg.blob_name.clone(),
        }
    }
}

impl From<DmMessage> for Message {
    fn from(msg: DmMessage) -> Self {
        match msg {
            DmMessage::Chat(chat) => Self::Chat(ChatMessage {
                sender_id: chat.sender_id,
                topic_id: chat.receiver_id,
                content: chat.content,
                timestamp: chat.timestamp,
                is_sent: chat.is_sent,
            }),
            DmMessage::Blob(blob) => Self::Blob(BlobMessage {
                sender_id: blob.sender_id,
                topic_id: blob.receiver_id,
                blob_hash: blob.blob_hash,
                blob_name: blob.blob_name,
                blob_size: blob.blob_size,
                timestamp: blob.timestamp,
                is_sent: blob.is_sent,
                blob_type: blob.blob_type,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveMessage {
    pub sender_id: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinMessage {
    pub sender_id: String,
    pub me: bool,
    pub timestamp: u64,
}

impl JoinMessage {
    #[must_use]
    pub const fn new(sender_id: String, timestamp: u64) -> Self {
        Self {
            sender_id,
            me: false,
            timestamp,
        }
    }

    #[must_use]
    pub fn new_me(timestamp: u64) -> Self {
        Self {
            sender_id: "You".to_string(),
            me: true,
            timestamp,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize, Eq)]
pub enum ConnectionStatus {
    Online,
    Offline(u64),
}

impl ConnectionStatus {
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn get_u64(&self) -> u64 {
        match self {
            Online => chrono::Utc::now().timestamp_millis() as u64,
            Offline(time) => time.to_owned(),
        }
    }
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

impl PartialOrd<Self> for ConnectionStatus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConnectionStatus {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Online, Online) => Ordering::Equal,
            (Online, Offline(_)) => Ordering::Greater,
            (Offline(_), Online) => Ordering::Less,
            (Offline(ts1), Offline(ts2)) => ts1.cmp(ts2),
        }
    }
}

impl Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Online => "Online".to_string(),
            Offline(timestamp) => format!("Offline since {timestamp}"),
        };
        write!(f, "{str}")
    }
}

impl Profile {
    #[must_use]
    pub fn new(id: &str, name: &str, avatar: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            avatar: Some(avatar.to_string()),
            last_connection: Online,
        }
    }

    #[must_use]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfileChat {
    pub profile: Profile,
    pub messages: Vec<DmMessage>,
    pub last_changed: u64,
}

impl ProfileChat {
    #[must_use]
    #[allow(clippy::cast_sign_loss)]
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            messages: Vec::new(),
            last_changed: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    #[must_use]
    pub fn last_message(&self) -> Option<String> {
        self.messages
            .last()
            .map(DmMessage::get_content)
    }

    pub fn add_dm_message(&mut self, message: DmChatMessage) {
        self.last_changed = message.timestamp;
        self.messages.push(DmMessage::Chat(message));
    }

    pub fn add_dm_blob_message(&mut self, message: DmBlobMessage) {
        self.last_changed = message.timestamp;
        self.messages.push(DmMessage::Blob(message));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnState {
    Topic,
    Contact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemovalType {
    Topic,
    Contact,
}

pub trait Controller {
    fn create_topic(&self, name: String);
    fn join_topic(&self, topic_id: String);
    fn leave_topic(&self, topic_id: String);
    fn remove_contact(&self, profile_id: String);
    fn send_message_to_topic(&self, ticket_id: String, message: String);
    fn modify_topic(&self, topic: Topic);
    fn modify_profile(&self, profile: Profile);
    fn send_message_to_user(&self, user_addr: String, message: String);
    fn connect_to_user(&self, user_id: String);
    fn send_blob_to_topic(
        &self,
        ticket_id: String,
        blob_data: FileData,
        name: String,
        blob_type: BlobType,
    );
    fn download_blob(&self, hash: String, user_id: String);
    fn get_from_storage(&self, hash: String, name: &str) -> Option<PathBuf>;
    fn has_blob(&self, image_hash: &str, image_name: &str) -> bool;
    fn send_blob_to_user(
        &self,
        user_addr: String,
        blob_data: FileData,
        name: String,
        blob_type: BlobType,
    );
    ///Returns an empty vector if the image could not be found or downloaded
    /// 
    /// # Errors
    /// 
    /// Return an error if it fails to get the blob or download
    fn get_or_download(&self, hash: &str, user_id: &str, name: &str) -> anyhow::Result<PathBuf>;
    fn get_media_url(&self, hash: &str, name: &str) -> String;
}
