use crate::client::DesktopClient;
use crate::utils::topics::save_topics_to_file;
use chrono::Utc;
use dioxus::core::spawn;
use dioxus::prelude::{Signal, WritableExt};
use dioxus::signals::ReadableExt;
use p2p::DmChatMessage as P2pDmChatMessage;
use p2p::{
    DmBlobMessage as P2pDmBlobMessage, DmJoinMessage, DmMessageTypes, DmProfileMetadataMessage,
    MessageTypes, Ticket, TopicMetadataMessage,
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::models::{AppState, ChatMessage, DmBlobMessage, DmChatMessage, Message};

pub fn handle_chat_message(mut state: Signal<AppState>, topic: &str, msg: &p2p::ChatMessage) {
    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let message = ChatMessage::new(
                msg.sender.to_string(),
                topic_obj.id.clone(),
                msg.content.clone(),
                msg.timestamp,
                false,
            );
            topic_obj.add_message(message);
        }
    });
}

pub fn handle_topic_metadata(
    mut state: Signal<AppState>,
    topic: &str,
    metadata: TopicMetadataMessage,
) -> Option<TopicMetadataMessage> {
    if let Some(existing_topic) = state().get_topic(topic) {
        if metadata.timestamp >= existing_topic.last_changed {
            state.with_mut(|s| {
                s.modify_topic_name(topic, &metadata.name);
                s.modify_topic_avatar(topic, metadata.avatar_url.clone());
                s.set_last_changed(topic, metadata.timestamp);
                let own_id = s.get_profile().id;
                let mut members = metadata.members;
                if !members.contains(&own_id) {
                    members.push(own_id);
                }
                s.set_topic_members(topic, members);
            });
            None
        } else if let Ok(ticket) = Ticket::from_str(topic) {
            Some(TopicMetadataMessage::new(
                ticket.topic,
                &existing_topic.name,
                existing_topic.avatar_url.clone(),
                existing_topic.last_changed,
                existing_topic.members.clone().into_iter().collect(),
            ))
        } else {
            None
        }
    } else {
        None
    }
}

#[allow(clippy::cast_sign_loss)]
pub fn handle_join_topic(
    mut state: Signal<AppState>,
    topic: &str,
    join_message: &p2p::JoinMessage,
) -> (
    Option<TopicMetadataMessage>,
    Option<p2p::TopicMessagesMessage>,
) {
    let metadata_to_send = state().get_all_topics().iter().find_map(|t| {
        let ticket = Ticket::from_str(&t.id).ok()?;
        if ticket.topic == join_message.topic {
            Some(TopicMetadataMessage::new(
                ticket.topic,
                &t.name,
                t.avatar_url.clone(),
                t.last_changed,
                t.members.clone().into_iter().collect(),
            ))
        } else {
            None
        }
    });

    let messages_to_send = state().get_topic(topic).and_then(|topic_obj| {
        let chat_messages: Vec<p2p::ChatMessage> = topic_obj
            .messages
            .iter()
            .filter_map(|msg| match msg {
                Message::Chat(chat_msg) => Some(chat_msg.to_p2p_message()),
                _ => None,
            })
            .collect();

        if chat_messages.is_empty() {
            None
        } else {
            Some(p2p::TopicMessagesMessage::new(
                join_message.topic,
                chat_messages,
            ))
        }
    });

    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let sender_id = join_message.endpoint.to_string();
            topic_obj.add_member(&sender_id);
            let message = ui::desktop::models::JoinMessage::new(
                sender_id,
                Utc::now().timestamp_millis() as u64,
            );
            topic_obj.add_join_message(message);
        }
    });

    (metadata_to_send, messages_to_send)
}

#[allow(clippy::cast_sign_loss)]
pub fn handle_leave_topic(mut state: Signal<AppState>, topic: &str, leave_msg: &p2p::LeaveMessage) {
    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let sender_id = leave_msg.endpoint.to_string();
            topic_obj.remove_member(&sender_id);
            let message = ui::desktop::models::LeaveMessage {
                sender_id,
                timestamp: Utc::now().timestamp_millis() as u64,
            };
            topic_obj.add_leave_message(message);
        }
    });
}

#[allow(clippy::cast_sign_loss)]
pub fn handle_disconnect_topic(
    mut state: Signal<AppState>,
    topic: &str,
    disconnect_msg: &p2p::DisconnectMessage,
) {
    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let message = ui::desktop::models::DisconnectMessage {
                sender_id: disconnect_msg.endpoint.to_string(),
                timestamp: Utc::now().timestamp_millis() as u64,
            };
            topic_obj.add_disconnect_message(message);
        }
    });
}

pub fn handle_blob_message(mut state: Signal<AppState>, topic: &str, msg: p2p::BlobMessage) {
    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let ui_blob_type = match msg.blob_type {
                p2p::messages::BlobType::Image => ui::desktop::models::BlobType::Image,
                p2p::messages::BlobType::BigImage => ui::desktop::models::BlobType::BigImage,
                p2p::messages::BlobType::File => ui::desktop::models::BlobType::File,
                p2p::messages::BlobType::Audio => ui::desktop::models::BlobType::Audio,
                p2p::messages::BlobType::Video => ui::desktop::models::BlobType::Video,
                p2p::messages::BlobType::Other => ui::desktop::models::BlobType::Other,
            };
            let message = ui::desktop::models::BlobMessage::new(
                msg.sender.to_string(),
                topic_obj.id.clone(),
                msg.hash.to_string(),
                msg.name,
                msg.size,
                msg.timestamp,
                false,
                ui_blob_type,
            );
            topic_obj.add_blob_message(message);
        }
    });
}

pub fn handle_topic_messages(
    mut state: Signal<AppState>,
    topic: &str,
    topic_messages_msg: &p2p::TopicMessagesMessage,
) -> Option<Vec<p2p::ChatMessage>> {
    state.with_mut(|s| {
        if let Some(topic_obj) = s.get_topic_mutable(topic) {
            let received_messages = topic_messages_msg
                .messages
                .iter()
                .map(ChatMessage::from_p2p_message)
                .collect::<Vec<ChatMessage>>();

            let existing_messages: Vec<ChatMessage> = topic_obj
                .messages
                .iter()
                .cloned()
                .filter_map(|msg| match msg {
                    Message::Chat(chat_msg) => Some(chat_msg),
                    _ => None,
                })
                .collect();

            for msg in &received_messages {
                if !existing_messages.contains(msg) {
                    topic_obj.add_message(msg.clone());
                }
            }
        }
    });

    state().get_topic(topic).and_then(|topic_obj| {
        let received_messages = topic_messages_msg
            .messages
            .iter()
            .map(ChatMessage::from_p2p_message)
            .collect::<Vec<ChatMessage>>();

        let existing_messages: Vec<ChatMessage> = topic_obj
            .messages
            .iter()
            .cloned()
            .filter_map(|msg| match msg {
                Message::Chat(chat_msg) => Some(chat_msg),
                _ => None,
            })
            .collect();

        let missing: Vec<p2p::ChatMessage> = existing_messages
            .iter()
            .filter(|msg| !received_messages.contains(msg))
            .map(ChatMessage::to_p2p_message)
            .collect();

        if missing.is_empty() {
            None
        } else {
            Some(missing)
        }
    })
}

#[allow(clippy::future_not_send)]
pub async fn process_message(
    client_ref: &Arc<Mutex<DesktopClient>>,
    state: Signal<AppState>,
    topic: String,
    message: MessageTypes,
) {
    match message {
        MessageTypes::Chat(msg) => {
            handle_chat_message(state, &topic, &msg);
        }
        MessageTypes::TopicMetadata(metadata) => {
            if let Some(metadata_to_send) = handle_topic_metadata(state, &topic, metadata)
                && let Err(e) = client_ref
                    .lock()
                    .await
                    .send(MessageTypes::TopicMetadata(metadata_to_send))
                    .await
            {
                eprintln!("Failed to send TopicMetadataMessage: {e}");
            }
        }
        MessageTypes::JoinTopic(join_message) => {
            let (metadata_to_send, messages_to_send) =
                handle_join_topic(state, &topic, &join_message);

            if let Some(metadata) = metadata_to_send
                && let Err(e) = client_ref
                    .lock()
                    .await
                    .send(MessageTypes::TopicMetadata(metadata))
                    .await
            {
                eprintln!("Failed to send TopicMetadataMessage: {e}");
            }

            if let Some(messages) = messages_to_send
                && let Err(e) = client_ref
                    .lock()
                    .await
                    .send(MessageTypes::TopicMessages(messages))
                    .await
            {
                eprintln!("Failed to send TopicMessagesMessage: {e}");
            }
        }
        MessageTypes::LeaveTopic(leave_msg) => {
            handle_leave_topic(state, &topic, &leave_msg);
        }
        MessageTypes::DisconnectTopic(disconnect_msg) => {
            handle_disconnect_topic(state, &topic, &disconnect_msg);
        }
        MessageTypes::TopicMessages(topic_messages_msg) => {
            if let Some(missing_messages) =
                handle_topic_messages(state, &topic, &topic_messages_msg)
                && let Ok(ticket) = Ticket::from_str(&topic)
            {
                let sync_message = p2p::TopicMessagesMessage::new(ticket.topic, missing_messages);

                if let Err(e) = client_ref
                    .lock()
                    .await
                    .send(MessageTypes::TopicMessages(sync_message))
                    .await
                {
                    eprintln!("Failed to send missing messages: {e}");
                }
            }
        }
        MessageTypes::Blob(image_message) => {
            handle_blob_message(state, &topic, image_message);
        }
    }
}

pub async fn collect_messages(
    client_ref: &Arc<Mutex<DesktopClient>>,
) -> Vec<(String, MessageTypes)> {
    let mut msgs = Vec::new();
    for (topic, receiver) in client_ref.lock().await.get_message_receiver() {
        while let Ok(message) = receiver.try_recv() {
            msgs.push((topic.clone(), message));
        }
    }
    msgs
}

#[allow(clippy::future_not_send)]
pub async fn process_all_messages(client_ref: &Arc<Mutex<DesktopClient>>, state: Signal<AppState>) {
    let messages = collect_messages(client_ref).await;

    for (topic, message) in messages {
        process_message(client_ref, state, topic, message).await;
    }

    let dm_messages = collect_dm_messages(client_ref).await;

    for (_sender, message) in dm_messages {
        process_dm_message(client_ref.clone(), state, message);
    }

    if let Err(e) = save_topics_to_file(&state().get_all_topics()) {
        eprintln!("Failed to save topics to file: {e}");
    }
}

pub trait P2PMessageConvert {
    fn from_p2p_message(msg: &p2p::ChatMessage) -> Self;
    fn to_p2p_message(&self) -> p2p::ChatMessage;
}

impl P2PMessageConvert for ChatMessage {
    fn from_p2p_message(msg: &p2p::ChatMessage) -> Self {
        Self::new(
            msg.sender.to_string(),
            msg.topic_id.to_string(),
            msg.content.clone(),
            msg.timestamp,
            false,
        )
    }

    fn to_p2p_message(&self) -> p2p::ChatMessage {
        let ticket = Ticket::from_str(&self.topic_id).expect("Invalid topic ID");
        p2p::ChatMessage::new(
            self.sender_id.parse().expect("Invalid sender ID"),
            self.content.clone(),
            self.timestamp,
            ticket.topic,
        )
    }
}

pub fn handle_dm_chat_message(mut state: Signal<AppState>, msg: &P2pDmChatMessage) {
    state.with_mut(|s| {
        let sender_id = msg.sender.to_string();
        let receiver_id = msg.receiver.to_string();

        let message = DmChatMessage::new(
            sender_id.clone(),
            receiver_id,
            msg.content.clone(),
            msg.timestamp,
            false,
        );
        s.add_dm_message(&sender_id, message);
    });
}

pub fn handle_dm_profile_metadata(mut state: Signal<AppState>, msg: DmProfileMetadataMessage) {
    state.with_mut(|s| {
        let profile_id = msg.id.to_string();

        let profile = ui::desktop::models::Profile {
            id: profile_id,
            name: msg.username,
            avatar: msg.avatar_url,
            last_connection: ui::desktop::models::ConnectionStatus::Offline(msg.last_connection),
        };

        s.modify_contact(profile);
    });
}

pub fn handle_dm_blob_message(mut state: Signal<AppState>, msg: P2pDmBlobMessage) {
    state.with_mut(|s| {
        let sender_id = msg.sender.to_string();
        let receiver_id = msg.receiver.to_string();

        let ui_blob_type = match msg.blob_type {
            p2p::messages::BlobType::Image => ui::desktop::models::BlobType::Image,
            p2p::messages::BlobType::BigImage => ui::desktop::models::BlobType::BigImage,
            p2p::messages::BlobType::File => ui::desktop::models::BlobType::File,
            p2p::messages::BlobType::Audio => ui::desktop::models::BlobType::Audio,
            p2p::messages::BlobType::Video => ui::desktop::models::BlobType::Video,
            p2p::messages::BlobType::Other => ui::desktop::models::BlobType::Other,
        };

        let message = DmBlobMessage::new(
            sender_id.clone(),
            receiver_id,
            msg.hash.to_string(),
            msg.name,
            msg.size,
            msg.timestamp,
            false,
            ui_blob_type,
        );
        s.add_dm_blob_message(&sender_id, message);
    });
}

pub fn handle_dm_join_petition(
    client_ref: Arc<Mutex<DesktopClient>>,
    mut state: Signal<AppState>,
    msg: &DmJoinMessage,
) {
    let petitioner_id = msg.petitioner.to_string();

    state.with_mut(|s| {
        if s.get_contact(&petitioner_id).is_none() {
            let profile = ui::desktop::models::Profile::new_with_id(&petitioner_id);
            s.add_contact(profile);
        }
    });

    let profile = state.read().get_profile();

    let endpoint_id = profile.id.parse().expect("Invalid endpoint ID");

    let profile_metadata = DmProfileMetadataMessage::new(
        endpoint_id,
        profile.name,
        profile.avatar,
        profile.last_connection.get_u64(),
    );

    spawn(async move {
        let client = client_ref.lock().await;

        if let Err(e) = client.connect_to_user(&petitioner_id).await {
            eprintln!("Failed to connect to petitioner: {e}");
            return;
        }

        if let Err(e) = client
            .send_dm(
                &petitioner_id,
                DmMessageTypes::ProfileMetadata(profile_metadata),
            )
            .await
        {
            eprintln!("Failed to send profile metadata: {e}");
        }
    });

    if let Err(e) = crate::utils::contacts::save_contacts(&state().get_all_contacts_chat()) {
        eprintln!("Failed to save contacts: {e}");
    }
}

pub async fn collect_dm_messages(
    client_ref: &Arc<Mutex<DesktopClient>>,
) -> Vec<(p2p::EndpointId, DmMessageTypes)> {
    let receiver_result = client_ref.lock().await.get_global_dm_receiver().await;

    let mut msgs = Vec::new();

    if let Ok(receiver) = receiver_result {
        while let Ok((sender, message)) = receiver.try_recv() {
            msgs.push((sender, message));
        }
    } else {
        eprintln!("Failed to get DM receiver");
    }
    msgs
}

pub fn process_dm_message(
    client_ref: Arc<Mutex<DesktopClient>>,
    state: Signal<AppState>,
    message: DmMessageTypes,
) {
    match message {
        DmMessageTypes::Chat(msg) => {
            handle_dm_chat_message(state, &msg);
        }
        DmMessageTypes::ProfileMetadata(msg) => {
            handle_dm_profile_metadata(state, msg);
        }
        DmMessageTypes::JoinPetition(msg) => {
            handle_dm_join_petition(client_ref, state, &msg);
        }
        DmMessageTypes::Blob(msg) => {
            handle_dm_blob_message(state, msg);
        }
    }
}
