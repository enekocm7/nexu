use crate::client::DesktopClient;
use crate::utils;
use crate::utils::topics::save_topics_to_file;
use base64::Engine;
use chrono::Utc;
use dioxus::prelude::{ReadableExt, Signal, WritableExt, spawn};
use p2p::{DmMessageTypes, DmProfileMetadataMessage, MessageTypes, Ticket, TopicMetadataMessage};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::models::{AppState, ChatMessage, DmChatMessage, Profile, ProfileChat, Topic};

pub struct AppController {
    app_state: Signal<AppState>,
    desktop_client: Arc<Mutex<DesktopClient>>,
}

impl AppController {
    pub fn new() -> Self {
        Self {
            app_state: Signal::new(AppState::new("Error")),
            desktop_client: Arc::new(Mutex::new(DesktopClient::new())),
        }
    }

    pub fn get_app_state(&self) -> Signal<AppState> {
        self.app_state
    }

    pub fn get_desktop_client(&self) -> Arc<Mutex<DesktopClient>> {
        Arc::clone(&self.desktop_client)
    }

    pub fn create_topic(&self, name: String) {
        let mut app_state = self.app_state;
        let desktop_client = Arc::clone(&self.desktop_client);

        spawn(async move {
            let result: Result<(), Error> = async {
                let ticket = desktop_client
                    .lock()
                    .await
                    .create_topic()
                    .await
                    .map_err(|e| Error::TopicCreation(e.to_string()))?;

                let mut topic = Topic::new(ticket.clone(), name, None);
                let profile = app_state.read().get_profile();
                topic.add_member(&profile.id);
                app_state.write().add_topic(&topic);

                save_topics_to_file(&app_state.read().get_all_topics())
                    .map_err(|_| Error::FileSave("Failed to save topics to file".to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to create topic: {}", e);
            }
        });
    }

    pub fn join_topic(&self, topic_id: String) {
        let mut app_state = self.get_app_state();
        let desktop_client = Arc::clone(&self.desktop_client);

        spawn(async move {
            let result: Result<(), Error> = async {
                if app_state().get_topic(&topic_id).is_some() {
                    return Ok(());
                }

                let mut topic = Topic::new_placeholder(topic_id.clone());

                let ticket_str = desktop_client
                    .lock()
                    .await
                    .join_topic(&topic_id)
                    .await
                    .map_err(|e| Error::TopicJoin(e.to_string()))?;

                let ticket = Ticket::from_str(&ticket_str)
                    .map_err(|_| Error::InvalidTicket("Invalid ticket string".to_string()))?;

                topic.add_join_message(ui::desktop::models::JoinMessage::new_me(
                    Utc::now().timestamp_millis() as u64,
                ));

                let profile = app_state.read().get_profile();
                topic.add_member(&profile.id);
                app_state.write().add_topic(&topic);

                save_topics_to_file(&app_state.read().get_all_topics())
                    .map_err(|_| Error::FileSave("Failed to save topics to file".to_string()))?;

                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

                let id = desktop_client
                    .lock()
                    .await
                    .peer_id()
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                desktop_client
                    .lock()
                    .await
                    .send(MessageTypes::JoinTopic(p2p::JoinMessage::new(
                        ticket.topic,
                        id,
                        Utc::now().timestamp_millis() as u64,
                    )))
                    .await
                    .map_err(|e| Error::MessageSend(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to join topic: {}", e);
            }
        });
    }

    pub fn leave_topic(&self, topic_id: String) {
        let mut app_state = self.app_state;
        let desktop_client = Arc::clone(&self.desktop_client);

        spawn(async move {
            let result: Result<(), Error> = async {
                let client_ref = desktop_client.clone();
                let mut client = client_ref.lock().await;

                let id = client
                    .peer_id()
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                let ticket = Ticket::from_str(&topic_id)
                    .map_err(|_| Error::InvalidTicket("Failed to parse topic_id".to_string()))?;

                client
                    .send(MessageTypes::LeaveTopic(p2p::LeaveMessage::new(
                        ticket.topic,
                        id,
                        Utc::now().timestamp_millis() as u64,
                    )))
                    .await
                    .map_err(|e| Error::MessageSend(e.to_string()))?;

                client
                    .leave_topic(&topic_id)
                    .await
                    .map_err(|e| Error::TopicLeave(e.to_string()))?;

                app_state.write().remove_topic(&topic_id);

                save_topics_to_file(&app_state.read().get_all_topics())
                    .map_err(|_| Error::FileSave("Failed to save topics to file".to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to leave topic: {}", e);
            }
        });
    }

    pub fn send_message_to_topic(&self, ticket_id: String, message: String) {
        let mut app_state = self.app_state;
        let desktop_client = Arc::clone(&self.desktop_client);
        let now = Utc::now().timestamp_millis() as u64;

        spawn(async move {
            let result: Result<(), Error> = async {
                let client_ref = desktop_client.clone();
                let (send_result, peer_id_result) = {
                    let client = client_ref.lock().await;
                    let msg = client
                        .get_chat_message(&ticket_id, &message)
                        .await
                        .map_err(|e| {
                            eprintln!("Failed to create chat message: {}", e);
                            Error::MessageCreation(e.to_string())
                        })?;

                    let send = client.send(MessageTypes::Chat(msg)).await;
                    let peer = client.peer_id().await;
                    (send, peer)
                };

                send_result.map_err(|e| {
                    eprintln!("Failed to send message: {}", e);
                    Error::MessageSend(e.to_string())
                })?;

                let peer_id = peer_id_result.map_err(|e| {
                    eprintln!("Failed to get peer_id: {}", e);
                    Error::PeerId(e.to_string())
                })?;

                app_state.with_mut(|state| {
                    if let Some(topic) = state.get_topic_mutable(&ticket_id) {
                        let msg = ChatMessage::new(
                            peer_id.to_string(),
                            ticket_id.clone(),
                            message.clone(),
                            now,
                            true,
                        );
                        topic.add_message(msg);
                    }
                });

                save_topics_to_file(&app_state().get_all_topics()).map_err(|_| {
                    eprintln!("Failed to save topics to file");
                    Error::FileSave("Failed to save topics to file".to_string())
                })?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to send message to topic {}: {}", ticket_id, e);
            }
        });
    }

    pub fn send_message_to_user(&self, user_addr: String, message: String) {
        let desktop_client = Arc::clone(&self.desktop_client);
        let mut app_state = self.get_app_state();

        spawn(async move {
            let result: Result<(), Error> = async {
                let client_ref = desktop_client.clone();
                let mut client = client_ref.lock().await;

                let msg = client
                    .get_dm_chat_message(&user_addr, &message)
                    .await
                    .map_err(|e| Error::MessageCreation(e.to_string()))?;

                client
                    .connect_to_user(&user_addr)
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                client
                    .send_dm(&user_addr, p2p::DmMessageTypes::Chat(msg))
                    .await
                    .map_err(|e| Error::MessageSend(e.to_string()))?;

                let peer_id = client
                    .peer_id()
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                let user_addr_clone = user_addr.clone();
                let message_clone = message.clone();

                app_state.with_mut(|state| {
                    let chat_msg = DmChatMessage::new(
                        peer_id.to_string(),
                        user_addr_clone.clone(),
                        message_clone,
                        Utc::now().timestamp_millis() as u64,
                        true,
                    );
                    state.add_dm_message(&user_addr_clone, chat_msg);
                });

                utils::contacts::save_contacts(&app_state.read().get_all_contacts_chat())
                    .map_err(|e| Error::ProfileSave(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to send message to user {u}: {e}", u = user_addr);
            }
        });
    }

    pub fn modify_topic(&self, topic: Topic) {
        let mut app_state = self.get_app_state();
        let desktop_client = Arc::clone(&self.desktop_client);

        spawn(async move {
            let result: Result<(), Error> = async {
                if let Some(ref avatar_url) = topic.avatar_url
                    && let Some(base64_data) = avatar_url.strip_prefix("data:")
                    && let Some(comma_pos) = base64_data.find(',')
                {
                    let base64_str = &base64_data[comma_pos + 1..];
                    if let Ok(decoded) =
                        base64::engine::general_purpose::STANDARD.decode(base64_str)
                    {
                        const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                        if decoded.len() > MAX_SIZE {
                            return Err(Error::ImageSizeExceeded);
                        }
                    }
                }

                app_state.write().modify_topic_name(&topic.id, &topic.name);
                app_state
                    .write()
                    .modify_topic_avatar(&topic.id, topic.avatar_url.clone());
                let time = app_state.write().set_last_changed_to_now(&topic.id);
                let ticket = Ticket::from_str(&topic.id).expect("Invalid ticket string");
                let update_message =
                    TopicMetadataMessage::new(ticket.topic, &topic.name, topic.avatar_url, time);

                if let Err(e) = desktop_client
                    .lock()
                    .await
                    .send(MessageTypes::TopicMetadata(update_message))
                    .await
                {
                    eprintln!("Failed to send update topic message: {}", e);
                }

                if save_topics_to_file(&app_state.read().get_all_topics()).is_err() {
                    return Err(Error::TopicModification(
                        "Failed to save topics to file".to_string(),
                    ));
                }

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to modify topic: {}", e);
            }
        });
    }

    pub fn modify_profile(&self, profile: Profile) {
        let mut app_state = self.app_state;
        let desktop_client = Arc::clone(&self.desktop_client);

        spawn(async move {
            let result: Result<(), Error> = async {
                app_state.with_mut(|state| {
                    state.set_profile_name(&profile.name);
                    state.set_profile_avatar(&profile.avatar);
                });

                let message = DmProfileMetadataMessage::new(
                    profile.id.parse().unwrap(),
                    profile.name.clone(),
                    profile.avatar.clone(),
                    profile.last_connection.get_u64(),
                );

                let contacts = app_state.read().get_all_contacts();

                for contact in contacts {
                    let client = desktop_client.lock().await;

                    if let Err(e) = client
                        .send_dm(
                            &contact.id,
                            DmMessageTypes::ProfileMetadata(message.clone()),
                        )
                        .await
                    {
                        eprintln!("Failed to send profile metadata to {}: {}", contact.id, e);
                    }
                }

                utils::contacts::save_profile(&profile)
                    .map_err(|e| Error::ProfileSave(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to save profile: {}", e);
            }
        });
    }

    pub fn connect_to_user(&self, user_id: String) {
        let desktop_client = Arc::clone(&self.desktop_client);
        let mut app_state = self.get_app_state();

        spawn(async move {
            let result: Result<(), Error> = async {
                let client_ref = desktop_client.clone();
                let mut client = client_ref.lock().await;

                client
                    .connect_to_user(&user_id)
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                let self_address = client
                    .peer_id()
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                let join_msg = p2p::DmJoinMessage::new(
                    self_address,
                    user_id.parse().map_err(|_| Error::InvalidPeerId)?,
                    Utc::now().timestamp_millis() as u64,
                );

                let user_id_clone = user_id.clone();
                app_state.with_mut(|state| {
                    if state.get_contact(&user_id_clone).is_none() {
                        let profile = Profile::new_with_id(&user_id_clone);
                        state.add_contact(profile);
                    }
                });

                client
                    .send_dm(&user_id, p2p::DmMessageTypes::JoinPetition(join_msg))
                    .await
                    .map_err(|e| Error::MessageSend(e.to_string()))?;

                let profile = app_state.read().get_profile();

                let msg = DmProfileMetadataMessage::new(
                    profile.id.parse().expect("id should be an EndpointId"),
                    profile.name,
                    profile.avatar,
                    profile.last_connection.get_u64(),
                );

                client
                    .send_dm(&user_id, p2p::DmMessageTypes::ProfileMetadata(msg))
                    .await
                    .map_err(|e| Error::MessageSend(e.to_string()))?;

                utils::contacts::save_contacts(&app_state.read().get_all_contacts_chat())
                    .map_err(|e| Error::ProfileSave(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to connect to user {}: {}", user_id, e);
            }
        });
    }

    pub fn reconnect_to_user(&self, chat: ProfileChat) {
        let desktop_client = Arc::clone(&self.desktop_client);
        let mut app_state = self.get_app_state();

        spawn(async move {
            let result: Result<(), Error> = async {
                app_state.write().add_contact_chat(chat.clone());

                desktop_client
                    .lock()
                    .await
                    .connect_to_user(&chat.profile.id)
                    .await
                    .map_err(|e| Error::PeerId(e.to_string()))?;

                utils::contacts::save_contacts(&app_state.read().get_all_contacts_chat())
                    .map_err(|e| Error::ProfileSave(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to reconnect to user {}: {}", chat.profile.id, e);
            }
        });
    }

    pub fn remove_contact(&self, profile_id: String) {
        let mut app_state = self.get_app_state();

        spawn(async move {
            let result: Result<(), Error> = async {
                app_state.write().remove_contact(&profile_id);

                utils::contacts::save_contacts(&app_state.read().get_all_contacts_chat())
                    .map_err(|e| Error::ProfileSave(e.to_string()))?;

                Ok(())
            }
            .await;

            if let Err(e) = result {
                eprintln!("Failed to remove contact {}: {}", profile_id, e);
            }
        });
    }
}

#[derive(Debug)]
pub enum Error {
    TopicCreation(String),
    TopicJoin(String),
    TopicLeave(String),
    TopicModification(String),
    FileSave(String),
    InvalidTicket(String),
    PeerId(String),
    InvalidPeerId,
    MessageSend(String),
    MessageCreation(String),
    ImageSizeExceeded,
    ProfileSave(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TopicCreation(msg) => write!(f, "Topic creation error: {}", msg),
            Error::TopicJoin(msg) => write!(f, "Topic join error: {}", msg),
            Error::TopicLeave(msg) => write!(f, "Topic leave error: {}", msg),
            Error::TopicModification(msg) => write!(f, "Topic modification error: {}", msg),
            Error::FileSave(msg) => write!(f, "File save error: {}", msg),
            Error::InvalidTicket(msg) => write!(f, "Invalid ticket: {}", msg),
            Error::PeerId(msg) => write!(f, "Peer ID error: {}", msg),
            Error::InvalidPeerId => write!(f, "Invalid peer ID"),
            Error::MessageSend(msg) => write!(f, "Message send error: {}", msg),
            Error::MessageCreation(msg) => write!(f, "Message creation error: {}", msg),
            Error::ImageSizeExceeded => write!(f, "Image size exceeds 512 KB limit"),
            Error::ProfileSave(msg) => write!(f, "Profile save error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
