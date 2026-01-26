use std::ffi::OsStr;
use crate::client::DesktopClient;
use crate::utils;
use crate::utils::topics::save_topics_to_file;
use base64::Engine;
use chrono::Utc;
use dioxus::html::FileData;
use dioxus::prelude::{ReadableExt, Signal, WritableExt};
use flume::{Receiver, Sender};
use futures_lite::StreamExt;
use p2p::{
    BlobTicket, DmMessageTypes, DmProfileMetadataMessage, DownloadProgressItem, EndpointAddr,
    EndpointId, Hash, MessageTypes, Raw, Ticket, TopicMetadataMessage,
};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::models::{
    AppState, BlobMessage, BlobType, ChatMessage, DmChatMessage, Profile, ProfileChat, Topic,
};

#[derive(Debug, Clone)]
pub enum Command {
    CreateTopic(String),
    JoinTopic(String),
    LeaveTopic(String),
    SendMessageToTopic {
        ticket_id: String,
        message: String,
    },
    SendBlobToTopic {
        ticket_id: String,
        blob_data: FileData,
        name: String,
        blob_type: BlobType,
    },
    DownloadBlob {
        blob_hash: String,
        user_id: String,
    },
    SendMessageToUser {
        user_addr: String,
        message: String,
    },
    ModifyTopic(Topic),
    ModifyProfile(Profile),
    ConnectToUser(String),
    RemoveContact(String),
}

pub struct AppController {
    desktop_client: Arc<Mutex<DesktopClient>>,
    progress_bar: Receiver<u64>,
    pub progress_bar_sender: Sender<u64>,
    command_sender: Sender<Command>,
    command_receiver: Receiver<Command>,
}

impl AppController {
    pub fn new() -> Self {
        let (progress_bar_sender, progress_bar) = flume::unbounded();
        let (command_sender, command_receiver) = flume::unbounded();
        Self {
            desktop_client: Arc::new(Mutex::new(DesktopClient::new())),
            progress_bar,
            progress_bar_sender,
            command_sender,
            command_receiver,
        }
    }

    pub fn get_desktop_client(&self) -> Arc<Mutex<DesktopClient>> {
        Arc::clone(&self.desktop_client)
    }

    pub fn get_progress_bar(&self) -> Receiver<u64> {
        self.progress_bar.clone()
    }

    pub fn get_command_receiver(&self) -> Receiver<Command> {
        self.command_receiver.clone()
    }

    fn send_command(&self, command: Command) {
        if let Err(e) = self.command_sender.send(command) {
            eprintln!("Failed to send command: {}", e);
        }
    }

    pub async fn process_command(
        command: Command,
        app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
        progress_sender: Sender<u64>,
    ) {
        match command {
            Command::CreateTopic(name) => {
                Self::do_create_topic(name, app_state, desktop_client).await;
            }
            Command::JoinTopic(topic_id) => {
                Self::do_join_topic(topic_id, app_state, desktop_client).await;
            }
            Command::LeaveTopic(topic_id) => {
                Self::do_leave_topic(topic_id, app_state, desktop_client).await;
            }
            Command::SendMessageToTopic { ticket_id, message } => {
                Self::do_send_message_to_topic(ticket_id, message, app_state, desktop_client).await;
            }
            Command::SendBlobToTopic {
                ticket_id,
                blob_data,
                name,
                blob_type,
            } => match blob_type {
                BlobType::Image => {
                    Self::do_send_image_to_topic(
                        ticket_id,
                        blob_data
                            .read_bytes()
                            .await
                            .expect("Failed to read image bytes")
                            .to_vec(),
                        name,
                        app_state,
                        desktop_client,
                        progress_sender,
                    )
                    .await;
                }
                BlobType::BigImage => {}
                BlobType::File => {
                    Self::do_send_blob_to_topic(
                        ticket_id,
                        blob_data,
                        name,
                        blob_type,
                        app_state,
                        desktop_client,
                        progress_sender,
                    )
                    .await;
                }
                BlobType::Audio => {}
                BlobType::Video => {}
                BlobType::Other => {}
            },
            Command::DownloadBlob {
                blob_hash,
                user_id,
            } => {
                Self::do_download_blob(&blob_hash, &user_id, desktop_client, progress_sender)
                    .await;
            }
            Command::SendMessageToUser { user_addr, message } => {
                Self::do_send_message_to_user(user_addr, message, app_state, desktop_client).await;
            }
            Command::ModifyTopic(topic) => {
                Self::do_modify_topic(topic, app_state, desktop_client).await;
            }
            Command::ModifyProfile(profile) => {
                Self::do_modify_profile(profile, app_state, desktop_client).await;
            }
            Command::ConnectToUser(user_id) => {
                Self::do_connect_to_user(user_id, app_state, desktop_client).await;
            }
            Command::RemoveContact(profile_id) => {
                Self::do_remove_contact(profile_id, app_state).await;
            }
        }
    }

    async fn do_create_topic(
        name: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
    }

    async fn do_join_topic(
        topic_id: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
    }

    async fn do_leave_topic(
        topic_id: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
    }

    async fn do_send_message_to_topic(
        ticket_id: String,
        message: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
        let now = Utc::now().timestamp_millis() as u64;

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
    }

    async fn do_send_image_to_topic(
        ticket_id: String,
        image_data: Vec<u8>,
        image_name: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
        progress_sender: Sender<u64>,
    ) {
        let now = Utc::now().timestamp_millis() as u64;

        let result: Result<(), Error> = async {
            let client_ref = desktop_client.clone();
            let client = client_ref.lock().await;

            let ticket = Ticket::from_str(&ticket_id)
                .map_err(|_| Error::InvalidTicket("Failed to parse ticket_id".to_string()))?;

            let peer_id = client
                .peer_id()
                .await
                .map_err(|e| Error::PeerId(e.to_string()))?;

            let add_stream = client
                .save_blob(image_data.clone())
                .await
                .map_err(|e| Error::BlobSave(e.to_string()))?;

            let mut stream = add_stream;
            let mut hash = None;
            while let Some(item) = stream.next().await {
                match item {
                    p2p::AddProgressItem::CopyProgress(_) => continue,
                    p2p::AddProgressItem::Size(_) => continue,
                    p2p::AddProgressItem::CopyDone => continue,
                    p2p::AddProgressItem::OutboardProgress(progress) => {
                        progress_sender
                            .send(progress)
                            .expect("Message to the channel should not return an error");
                        continue;
                    }
                    p2p::AddProgressItem::Done(temp_tag) => {
                        hash = Some(temp_tag.hash());
                        progress_sender
                            .send(u64::MAX)
                            .expect("Message to the channel should not return an error");
                        break;
                    }
                    p2p::AddProgressItem::Error(error) => {
                        return Err(Error::BlobSave(error.to_string()));
                    }
                }
            }

            let hash =
                hash.ok_or_else(|| Error::BlobSave("Failed to get hash from stream".to_string()))?;

            let msg = p2p::BlobMessage::new(
                ticket.topic,
                peer_id,
                image_name.clone(),
                image_data.len() as u64,
                hash,
                now,
                p2p::messages::BlobType::Image,
            );

            client.send(MessageTypes::Blob(msg)).await.map_err(|e| {
                eprintln!("Failed to send message: {}", e);
                Error::MessageSend(e.to_string())
            })?;

            app_state.with_mut(|state| {
                if let Some(topic) = state.get_topic_mutable(&ticket_id) {
                    let image_msg = BlobMessage::new(
                        peer_id.to_string(),
                        ticket_id.clone(),
                        hash.to_string(),
                        image_name,
                        image_data.len() as u64,
                        now,
                        true,
                        BlobType::Image,
                    );
                    topic.add_image_message(image_msg);
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
            eprintln!("Failed to send image to topic {}: {}", ticket_id, e);
        }
    }

    async fn do_send_blob_to_topic(
        ticket_id: String,
        blob_data: FileData,
        blob_name: String,
        blob_type: BlobType,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
        progress_sender: Sender<u64>,
    ) {
        let now = Utc::now().timestamp_millis() as u64;

        let result: Result<(), Error> = async {
            let client_ref = desktop_client.clone();
            let client = client_ref.lock().await;

            let ticket = Ticket::from_str(&ticket_id)
                .map_err(|_| Error::InvalidTicket("Failed to parse ticket_id".to_string()))?;

            let peer_id = client
                .peer_id()
                .await
                .map_err(|e| Error::PeerId(e.to_string()))?;

            let add_stream = client
                .save_blob_from_path(blob_data.path())
                .await
                .map_err(|e| Error::BlobSave(e.to_string()))?;

            let mut stream = add_stream;
            let mut hash = None;
            while let Some(item) = stream.next().await {
                match item {
                    p2p::AddProgressItem::CopyProgress(_) => continue,
                    p2p::AddProgressItem::Size(_) => continue,
                    p2p::AddProgressItem::CopyDone => continue,
                    p2p::AddProgressItem::OutboardProgress(progress) => {
                        progress_sender
                            .send(progress)
                            .expect("Message to the channel should not return an error");
                        continue;
                    }
                    p2p::AddProgressItem::Done(temp_tag) => {
                        hash = Some(temp_tag.hash());
                        progress_sender
                            .send(u64::MAX)
                            .expect("Message to the channel should not return an error");
                        break;
                    }
                    p2p::AddProgressItem::Error(error) => {
                        return Err(Error::BlobSave(error.to_string()));
                    }
                }
            }

            let hash =
                hash.ok_or_else(|| Error::BlobSave("Failed to get hash from stream".to_string()))?;

            let p2p_blob_type = match blob_type {
                BlobType::File => p2p::messages::BlobType::File,
                _ => p2p::messages::BlobType::Other,
            };

            let msg = p2p::BlobMessage::new(
                ticket.topic,
                peer_id,
                blob_name.clone(),
                blob_data.size(),
                hash,
                now,
                p2p_blob_type,
            );

            client.send(MessageTypes::Blob(msg)).await.map_err(|e| {
                eprintln!("Failed to send message: {}", e);
                Error::MessageSend(e.to_string())
            })?;

            app_state.with_mut(|state| {
                if let Some(topic) = state.get_topic_mutable(&ticket_id) {
                    let image_msg = BlobMessage::new(
                        peer_id.to_string(),
                        ticket_id.clone(),
                        hash.to_string(),
                        blob_name.clone(),
                        blob_data.size(),
                        now,
                        true,
                        BlobType::File,
                    );
                    topic.add_image_message(image_msg);
                }
            });

            Ok(())
        }
        .await;

        if let Err(e) = result {
            eprintln!("Failed to send blob to topic {}: {}", ticket_id, e);
        }
    }

    async fn do_download_blob(
        blob_hash: &str,
        user_id: &str,
        desktop_client: Arc<Mutex<DesktopClient>>,
        progress_sender: Sender<u64>,
    ) {
        let result: Result<(), Error> = async {
            let hash = blob_hash
                .parse::<Hash>()
                .expect("image hash should be parseable");
            let endpoint_id = EndpointId::from_str(user_id)
                .map_err(|e| Error::InvalidUserId(format!("Invalid user ID: {e}")))?;
            let addr = EndpointAddr::from(endpoint_id);
            let ticket = BlobTicket::new(addr, hash, Raw);

            let progress = desktop_client
                .lock()
                .await
                .download_blob(&ticket)
                .await
                .map_err(|e| Error::DownloadBlob(format!("Failed to start blob download: {e}")))?;

            let mut stream = progress.stream().await.map_err(|e| {
                Error::DownloadBlob(format!("Failed to get download progress stream: {e}"))
            })?;

            while let Some(item) = stream.next().await {
                match item {
                    DownloadProgressItem::Progress(progress) => {
                        progress_sender
                            .send(progress)
                            .expect("Message to the channel should not return an error");
                    }
                    DownloadProgressItem::Error(e) => {
                        let _ = progress_sender.send(u64::MAX);
                        return Err(Error::DownloadBlob(format!(
                            "Error during blob download: {}",
                            e
                        )));
                    }
                    DownloadProgressItem::TryProvider { id, request } => {
                        println!("Trying provider {} for request {:?}", id, request);
                    }
                    DownloadProgressItem::ProviderFailed { id, request } => {
                        eprintln!("Provider {} failed for request {:?}", id, request);
                    }
                    DownloadProgressItem::PartComplete { request } => {
                        println!("Part complete for request {:?}", request);
                    }
                    DownloadProgressItem::DownloadError => {
                        let _ = progress_sender.send(u64::MAX);
                        return Err(Error::DownloadBlob("Download error occurred".to_string()));
                    }
                }
            }

            let _ = progress_sender.send(u64::MAX);

            Ok(())
        }
        .await;

        if let Err(e) = result {
            eprintln!("Failed to download blob {}: {}", blob_hash, e);
        }
    }

    pub fn get_blob_from_storage(&self, hash: Hash, extension: impl AsRef<OsStr>) -> Option<PathBuf> {
        let desktop_client = Arc::clone(&self.desktop_client);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { desktop_client.lock().await.get_blob_path(hash, extension).await.ok() })
        })
    }

    pub fn has_blob_impl(&self, hash: &str, extension: impl AsRef<OsStr>) -> bool {
        let hash = match hash.parse::<Hash>() {
            Ok(h) => h,
            Err(_) => return false,
        };

        let desktop_client = Arc::clone(&self.desktop_client);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                desktop_client.lock().await.get_blob_path(hash, extension).await.is_ok()
            })
        })
    }

    async fn do_send_message_to_user(
        user_addr: String,
        message: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
                .send_dm(&user_addr, DmMessageTypes::Chat(msg))
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
    }

    async fn do_modify_topic(
        topic: Topic,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
        let result: Result<(), Error> = async {
            if let Some(ref avatar_url) = topic.avatar_url
                && let Some(base64_data) = avatar_url.strip_prefix("data:")
                && let Some(comma_pos) = base64_data.find(',')
            {
                let base64_str = &base64_data[comma_pos + 1..];
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(base64_str) {
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
            let update_message = TopicMetadataMessage::new(
                ticket.topic,
                &topic.name,
                topic.avatar_url,
                time,
                topic.members.into_iter().collect(),
            );

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
    }

    async fn do_modify_profile(
        profile: Profile,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
    }

    async fn do_connect_to_user(
        user_id: String,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
                .send_dm(&user_id, DmMessageTypes::JoinPetition(join_msg))
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
                .send_dm(&user_id, DmMessageTypes::ProfileMetadata(msg))
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
    }

    async fn do_reconnect_to_user(
        chat: ProfileChat,
        mut app_state: Signal<AppState>,
        desktop_client: Arc<Mutex<DesktopClient>>,
    ) {
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
    }

    async fn do_remove_contact(profile_id: String, mut app_state: Signal<AppState>) {
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
    }

    pub async fn reconnect_to_user_async(&self, app_state: Signal<AppState>, chat: ProfileChat) {
        Self::do_reconnect_to_user(chat, app_state, Arc::clone(&self.desktop_client)).await;
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
    BlobSave(String),
    DownloadBlob(String),
    InvalidUserId(String),
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
            Error::BlobSave(msg) => write!(f, "Blob save error: {}", msg),
            Error::DownloadBlob(msg) => write!(f, "Download blob error: {}", msg),
            Error::InvalidUserId(id) => write!(f, "Invalid user ID: {}", id),
        }
    }
}

impl std::error::Error for Error {}

impl ui::desktop::models::Controller for AppController {
    fn create_topic(&self, name: String) {
        self.send_command(Command::CreateTopic(name));
    }

    fn join_topic(&self, topic_id: String) {
        self.send_command(Command::JoinTopic(topic_id));
    }

    fn leave_topic(&self, topic_id: String) {
        self.send_command(Command::LeaveTopic(topic_id));
    }

    fn remove_contact(&self, profile_id: String) {
        self.send_command(Command::RemoveContact(profile_id));
    }

    fn send_message_to_topic(&self, ticket_id: String, message: String) {
        self.send_command(Command::SendMessageToTopic { ticket_id, message });
    }

    fn modify_topic(&self, topic: Topic) {
        self.send_command(Command::ModifyTopic(topic));
    }

    fn modify_profile(&self, profile: Profile) {
        self.send_command(Command::ModifyProfile(profile));
    }

    fn send_message_to_user(&self, user_addr: String, message: String) {
        self.send_command(Command::SendMessageToUser { user_addr, message });
    }

    fn connect_to_user(&self, user_id: String) {
        self.send_command(Command::ConnectToUser(user_id));
    }

    fn send_blob_to_topic(
        &self,
        ticket_id: String,
        blob_data: FileData,
        name: String,
        blob_type: BlobType,
    ) {
        self.send_command(Command::SendBlobToTopic {
            ticket_id,
            blob_data,
            name,
            blob_type,
        });
    }

    fn download_image(&self, image_hash: String, user_id: String) {
        self.send_command(Command::DownloadBlob {
            blob_hash: image_hash,
            user_id,
        });
    }

    fn get_image_from_storage(&self, image_hash: String, image_name: &str) -> Option<PathBuf> {
        let extension = image_name
            .split('.')
            .next_back()
            .unwrap_or("");
        self.get_blob_from_storage(image_hash.parse().expect("Image hash should be parseable"), extension)
    }

    fn has_blob(&self, image_hash: &str, image_name: &str) -> bool {
        let extension = image_name
            .split('.')
            .next_back()
            .unwrap_or("");
        self.has_blob_impl(image_hash, extension)
    }

    fn get_or_download_image(&self, image_hash: &str, user_id: &str, image_name: &str) -> anyhow::Result<PathBuf> {
        let hash = image_hash
            .parse::<Hash>()
            .expect("Image hash should be parseable");

        let extension = image_name.split('.').next_back().unwrap_or("");

        if let Some(data) = self.get_blob_from_storage(hash, extension) {
            return Ok(data);
        }
        let desktop_client = Arc::clone(&self.desktop_client);
        let progress_sender = self.progress_bar_sender.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let endpoint_id =
                    EndpointId::from_str(user_id).expect("Endpoint ID should be parseable");
                let addr = EndpointAddr::from(endpoint_id);
                let ticket = BlobTicket::new(addr, hash, Raw);

                let progress = desktop_client
                    .lock()
                    .await
                    .download_blob(&ticket)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to start blob download: {}", e))?;

                let mut stream = progress.stream().await.map_err(|e| {
                    anyhow::anyhow!("Failed to get download progress stream: {}", e)
                })?;

                while let Some(item) = stream.next().await {
                    match item {
                        DownloadProgressItem::Progress(progress) => {
                            let _ = progress_sender.send(progress);
                        }
                        DownloadProgressItem::Error(e) => {
                            let _ = progress_sender.send(u64::MAX);
                            return Err(anyhow::anyhow!(e));
                        }
                        DownloadProgressItem::DownloadError => {
                            let _ = progress_sender.send(u64::MAX);
                            return Err(anyhow::anyhow!("Download error occurred"));
                        }
                        DownloadProgressItem::PartComplete { request } => {
                            println!("Part complete for request {:?}", request);
                        }
                        DownloadProgressItem::TryProvider { id, request } => {
                            println!("Trying provider {} for request {:?}", id, request);
                        }
                        DownloadProgressItem::ProviderFailed { id, request } => {
                            eprintln!("Provider {} failed for request {:?}", id, request);
                        }
                    }
                }

                let _ = progress_sender.send(u64::MAX);

                desktop_client
                    .lock()
                    .await
                    .get_blob_path(hash, extension)
                    .await
            })
        })
    }
}
