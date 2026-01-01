mod client;
mod utils;

use crate::client::DesktopClient;
use crate::utils::contacts::load_profile;
use crate::utils::topics::{load_topics_from_file, save_topics_to_file};
use base64::Engine;
use chrono::Utc;
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder, use_wry_event_handler};
use dioxus::prelude::*;
use p2p::{MessageTypes, Ticket, TopicMetadataMessage};
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::desktop_web_components::Desktop;
use ui::desktop::models::{AppState, ChatMessage, Message, Profile, Topic};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    LaunchBuilder::new()
        .with_cfg(
            Config::default().with_menu(None).with_window(
                WindowBuilder::new()
                    .with_title("Nexu")
                    .with_window_icon(load_icon())
                    .with_min_inner_size(LogicalSize::new(800, 600)),
            ),
        )
        .launch(App);
}

async fn join_topic_internal(
    desktop_client: &Arc<Mutex<DesktopClient>>,
    mut app_state: Signal<AppState>,
    mut topic: Topic,
) -> Result<(), Box<dyn Error>> {
    let join_result = desktop_client.lock().await.join_topic(&topic.id).await;
    let mut state = app_state.write();

    match join_result {
        Ok(ticket_str) => {
            let ticket = Ticket::from_str(&ticket_str).expect("Invalid ticket string");

            topic.add_join_message(ui::desktop::models::JoinMessage::new_me(
                Utc::now().timestamp_millis() as u64,
            ));

            state.add_topic(&topic);

            if save_topics_to_file(&state.get_all_topics()).is_err() {
                eprintln!("Failed to save topics to file");
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            let id = desktop_client
                .lock()
                .await
                .peer_id()
                .await?
                .parse()
                .expect("Invalid peer id");

            desktop_client
                .lock()
                .await
                .send(MessageTypes::JoinTopic(p2p::JoinMessage::new(
                    ticket.topic,
                    id,
                    Utc::now().timestamp_millis() as u64,
                )))
                .await
                .expect("Failed to send JoinTopic message");

            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to join topic: {e}");
            Err(e.into())
        }
    }
}

#[component]
fn App() -> Element {
    let mut app_state = use_signal(|| AppState::new("temp"));
    let desktop_client = use_signal(|| Arc::new(Mutex::new(DesktopClient::new())));

    let on_modify_topic = move |topic: Topic| {
        spawn(async move {
            if let Some(ref avatar_url) = topic.avatar_url
                && let Some(base64_data) = avatar_url.strip_prefix("data:")
                && let Some(comma_pos) = base64_data.find(',')
            {
                let base64_str = &base64_data[comma_pos + 1..];
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(base64_str) {
                    const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                    if decoded.len() > MAX_SIZE {
                        eprintln!("Image size exceeds 512 KB limit, rejecting update");
                        return;
                    }
                }
            }
            let mut state = app_state.write();
            state.modify_topic_name(&topic.id, &topic.name);
            state.modify_topic_avatar(&topic.id, topic.avatar_url.clone());
            let time = state.set_last_changed_to_now(&topic.id);
            let ticket = Ticket::from_str(&topic.id).expect("Invalid ticket string");
            let update_message =
                TopicMetadataMessage::new(ticket.topic, &topic.name, topic.avatar_url, time);
            if let Err(e) = desktop_client
                .read()
                .lock()
                .await
                .send(MessageTypes::TopicMetadata(update_message))
                .await
            {
                eprintln!("Failed to send update topic message: {}", e);
            }
            if save_topics_to_file(&state.get_all_topics()).is_err() {
                eprintln!("Failed to save topics to file");
            }
        });
    };

    let on_create_topic = move |name: String| {
        spawn(async move {
            let ticket = desktop_client.read().lock().await.create_topic().await;
            match ticket {
                Ok(ticket) => {
                    let mut state = app_state.write();
                    let topic = Topic::new(ticket.clone(), name, None);
                    state.add_topic(&topic);

                    if save_topics_to_file(&state.get_all_topics()).is_err() {
                        eprintln!("Failed to save topics to file");
                    }
                }
                Err(e) => eprintln!("Failed to create topic: {}", e),
            }
        });
    };

    let on_join_topic = move |topic_id: String| {
        spawn(async move {
            let client_ref = desktop_client.read().clone();
            if app_state.read().get_topic_immutable(&topic_id).is_some() {
                return;
            }
            let topic = Topic::new_placeholder(topic_id.clone());
            let _ = join_topic_internal(&client_ref, app_state, topic).await;
        });
    };

    let on_leave_topic = move |topic_id: String| {
        spawn(async move {
            let client_ref = desktop_client.read().clone();
            let mut client = client_ref.lock().await;
            let id = client
                .peer_id()
                .await
                .expect("Failed to get peer_id")
                .parse()
                .expect("Failed to parse peer_id");

            let ticket = Ticket::from_str(&topic_id).expect("Failed to parse topic_id");

            client
                .send(MessageTypes::LeaveTopic(p2p::LeaveMessage::new(
                    ticket.topic,
                    id,
                    Utc::now().timestamp_millis() as u64,
                )))
                .await
                .expect("Failed to send LeaveTopic message");

            let leave_result = client.leave_topic(&topic_id).await;

            match leave_result {
                Ok(_) => {
                    let mut state = app_state.write();
                    state.remove_topic(&topic_id);

                    if save_topics_to_file(&state.get_all_topics()).is_err() {
                        eprintln!("Failed to save topics to file");
                    }
                }
                Err(e) => eprintln!("Failed to leave topic: {}", e),
            }
        });
    };

    let on_send_message = move |(ticket_id, message): (String, String)| {
        let now = Utc::now().timestamp_millis() as u64;
        spawn(async move {
            let client_ref = desktop_client.read().clone();
            let (send_result, peer_id_result) = {
                let client = client_ref.lock().await;
                let message = client
                    .get_chat_message(&ticket_id, &message)
                    .await
                    .expect("Failed to create chat message");
                let send = client.send(MessageTypes::Chat(message)).await;
                let peer = client.peer_id().await;
                (send, peer)
            };

            match (send_result, peer_id_result) {
                (Ok(_), Ok(peer_id)) => {
                    let mut state = app_state.write();
                    if let Some(topic) = state.get_topic(&ticket_id) {
                        let msg = ChatMessage::new(peer_id, ticket_id, message, now, true);
                        topic.add_message(msg);

                        if save_topics_to_file(&state.get_all_topics()).is_err() {
                            eprintln!("Failed to save topics to file");
                        }
                    }
                }
                (Err(e), _) => {
                    eprintln!("Failed to send message to topic {}: {}", ticket_id, e);
                }
                (_, Err(e)) => {
                    eprintln!("Failed to get peer_id: {}", e);
                }
            }
        });
    };

    let on_modify_profile = move |profile: Profile| {
        let mut state = app_state.write();
        state.set_profile_name(&profile.name);
        state.set_profile_avatar(&profile.avatar);
        if let Err(e) = utils::contacts::save_profile(&profile) {
            eprintln!("Failed to save profile: {e}")
        }
    };

    use_effect(move || {
        let client_ref = desktop_client.read().clone();
        spawn(async move {
            if let Err(e) = client_ref.lock().await.initialize().await {
                eprintln!("Failed to initialize DesktopClient: {}", e);
                return;
            }

            let peer_id = match client_ref.lock().await.peer_id().await {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("Failed to get peer_id: {}", e);
                    return;
                }
            };

            if let Ok(profile) = load_profile() {
                let mut state = app_state.write();
                state.set_profile_id(&profile.id);
                state.set_profile_name(&profile.name);
                state.set_profile_avatar(&profile.avatar);
                state.set_profile_last_connection_to_now();
            } else {
                let mut state = app_state.write();
                state.set_profile_id(&peer_id);
                state.set_profile_name(&peer_id);
            }

            if let Ok(loaded_topics) = load_topics_from_file() {
                for topic in loaded_topics {
                    let client_ref = desktop_client.read().clone();
                    spawn(async move {
                        let _ = join_topic_internal(&client_ref, app_state, topic).await;
                    });
                }
            }

            loop {
                let messages: Vec<(String, MessageTypes)> = {
                    let mut client = client_ref.lock().await;
                    let mut msgs = Vec::new();
                    for (topic, receiver) in client.get_message_receiver() {
                        while let Ok(message) = receiver.try_recv() {
                            msgs.push((topic.to_string(), message));
                        }
                    }
                    msgs
                };

                let had_messages = !messages.is_empty();

                for (topic, message) in messages {
                    match message {
                        MessageTypes::Chat(msg) => {
                            let mut state = app_state.write();
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ChatMessage::new(
                                    msg.sender.to_string(),
                                    topic_obj.id.clone(),
                                    msg.content,
                                    msg.timestamp,
                                    false,
                                );
                                topic_obj.add_message(message);
                            }
                        }
                        MessageTypes::TopicMetadata(metadata) => {
                            let should_send = {
                                let mut state = app_state.write();
                                if let Some(existing_topic) = state.get_topic(&topic) {
                                    if metadata.timestamp >= existing_topic.last_changed {
                                        state.modify_topic_name(&topic, &metadata.name);
                                        state.modify_topic_avatar(&topic, metadata.avatar_url);
                                        state.set_last_changed(&topic, metadata.timestamp);
                                        None
                                    } else if let Ok(ticket) = Ticket::from_str(&topic) {
                                        Some(TopicMetadataMessage::new(
                                            ticket.topic,
                                            &existing_topic.name,
                                            existing_topic.avatar_url.clone(),
                                            existing_topic.last_changed,
                                        ))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };
                            if let Some(metadata) = should_send
                                && let Err(e) = client_ref
                                    .lock()
                                    .await
                                    .send(MessageTypes::TopicMetadata(metadata))
                                    .await
                            {
                                eprintln!("Failed to send TopicMetadataMessage: {}", e);
                            }
                        }
                        MessageTypes::JoinTopic(join_message) => {
                            let metadata_to_send = {
                                let state = app_state.read();
                                state.get_all_topics().iter().find_map(|topic| {
                                    let ticket = Ticket::from_str(&topic.id).ok()?;
                                    if ticket.topic == join_message.topic {
                                        Some(TopicMetadataMessage::new(
                                            ticket.topic,
                                            &topic.name,
                                            topic.avatar_url.clone(),
                                            topic.last_changed,
                                        ))
                                    } else {
                                        None
                                    }
                                })
                            };
                            if let Some(message) = metadata_to_send
                                && let Err(e) = client_ref
                                    .lock()
                                    .await
                                    .send(MessageTypes::TopicMetadata(message))
                                    .await
                            {
                                eprintln!("Failed to send TopicMetadataMessage: {}", e);
                            }

                            let messages_to_send = {
                                let state = app_state.read();
                                if let Some(topic_obj) = state.get_topic_immutable(&topic) {
                                    let chat_messages: Vec<p2p::ChatMessage> = topic_obj
                                        .messages
                                        .iter()
                                        .filter_map(|msg| match msg {
                                            Message::Chat(chat_msg) => Some(chat_msg.to_message()),
                                            _ => None,
                                        })
                                        .collect();

                                    if !chat_messages.is_empty() {
                                        Some(p2p::TopicMessagesMessage::new(
                                            join_message.topic,
                                            chat_messages,
                                        ))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            if let Some(messages) = messages_to_send
                                && let Err(e) = client_ref
                                    .lock()
                                    .await
                                    .send(MessageTypes::TopicMessages(messages))
                                    .await
                            {
                                eprintln!("Failed to send TopicMessagesMessage: {}", e);
                            }

                            let mut state = app_state.write();
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::JoinMessage::new(
                                    join_message.endpoint.to_string(),
                                    Utc::now().timestamp_millis() as u64,
                                );
                                topic_obj.add_join_message(message);
                            }
                        }
                        MessageTypes::LeaveTopic(message) => {
                            let mut state = app_state.write();
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::LeaveMessage {
                                    sender_id: message.endpoint.to_string(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_leave_message(message);
                            }
                        }
                        MessageTypes::DisconnectTopic(message) => {
                            let mut state = app_state.write();
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::DisconnectMessage {
                                    sender_id: message.endpoint.to_string(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_disconnect_message(message);
                            }
                        }
                        MessageTypes::TopicMessages(message) => {
                            if message.messages.is_empty() {
                                continue;
                            }

                            let mut state = app_state.write();
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let received_messages = message
                                    .messages
                                    .iter()
                                    .map(ChatMessage::from_message)
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

                                let missing_messages: Vec<p2p::ChatMessage> = existing_messages
                                    .iter()
                                    .filter(|msg| !received_messages.contains(msg))
                                    .map(|msg| msg.to_message())
                                    .collect();

                                if !missing_messages.is_empty() {
                                    let client_ref = desktop_client.read().clone();
                                    let sync_message = p2p::TopicMessagesMessage::new(
                                        Ticket::from_str(&topic)
                                            .expect("Failed to parse topic ID")
                                            .topic,
                                        missing_messages,
                                    );

                                    if let Err(e) = client_ref
                                        .lock()
                                        .await
                                        .send(MessageTypes::TopicMessages(sync_message))
                                        .await
                                    {
                                        eprintln!("Failed to send missing messages: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                if had_messages && save_topics_to_file(&app_state.read().get_all_topics()).is_err()
                {
                    eprintln!("Failed to save topics to file");
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    });

    use_wry_event_handler(move |event, _| {
        if let dioxus::desktop::tao::event::Event::WindowEvent { event, .. } = event
            && event == &dioxus::desktop::tao::event::WindowEvent::CloseRequested
        {
            let client_ref = desktop_client.read().clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let client = client_ref.lock().await;
                    let id = client
                        .peer_id()
                        .await
                        .expect("Failed to get peer_id")
                        .parse()
                        .expect("Failed to parse peer_id");

                    let all_topics = app_state.read().get_all_topics();

                    for topic in all_topics.iter() {
                        let ticket = Ticket::from_str(&topic.id).expect("Failed to parse topic_id");

                        let message = MessageTypes::DisconnectTopic(p2p::DisconnectMessage::new(
                            ticket.topic,
                            id,
                            Utc::now().timestamp_millis() as u64,
                        ));
                        if let Err(e) = client.send(message).await {
                            eprintln!("Failed to send DisconnectTopic message: {}", e);
                        }
                    }
                });
            });
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Desktop {
            app_state,
            on_create_topic,
            on_join_topic,
            on_leave_topic,
            on_send_message,
            on_modify_topic,
            on_modify_profile,
        }
    }
}

fn load_icon() -> Option<Icon> {
    let icon_bytes = include_bytes!("../assets/logo.png");
    let img = image::load_from_memory(icon_bytes).expect("Failed to load icon image");

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

trait FromP2PMessage {
    fn from_message(msg: &p2p::ChatMessage) -> Self;
    fn to_message(&self) -> p2p::ChatMessage;
}

impl FromP2PMessage for ChatMessage {
    fn from_message(msg: &p2p::ChatMessage) -> Self {
        ChatMessage::new(
            msg.sender.to_string(),
            msg.topic_id.to_string(),
            msg.content.clone(),
            msg.timestamp,
            false,
        )
    }

    fn to_message(&self) -> p2p::ChatMessage {
        let ticket = Ticket::from_str(&self.topic_id).expect("Invalid topic ID");
        p2p::ChatMessage::new(
            self.sender_id.parse().expect("Invalid sender ID"),
            self.content.clone(),
            self.timestamp,
            ticket.topic,
        )
    }
}
