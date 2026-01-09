mod client;
mod controller;
mod utils;

use crate::client::DesktopClient;
use crate::utils::contacts::load_profile;
use crate::utils::topics::{load_topics_from_file, save_topics_to_file};
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

    match join_result {
        Ok(ticket_str) => {
            let ticket = Ticket::from_str(&ticket_str).expect("Invalid ticket string");

            topic.add_join_message(ui::desktop::models::JoinMessage::new_me(
                Utc::now().timestamp_millis() as u64,
            ));
            let profile = app_state().get_profile();

            topic.add_member(&profile.id);

            app_state.with_mut(|state| state.add_topic(&topic));

            if save_topics_to_file(&app_state().get_all_topics()).is_err() {
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
    let controller = use_signal(controller::AppController::new);

    use_effect(move || {
        let client_ref = controller.read().get_desktop_client();
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

            if let Ok(mut profile) = load_profile() {
                if peer_id != profile.id {
                    profile.id = peer_id.clone();
                    utils::contacts::save_profile(&profile).unwrap_or_else(|e| {
                        eprintln!("Failed to update profile ID: {e}");
                    });
                }
                let mut state = controller.read().get_app_state();
                state.write().set_profile_id(&peer_id);
                state.write().set_profile_name(&profile.name);
                state.write().set_profile_avatar(&profile.avatar);
                state.write().set_profile_last_connection_to_now();
            } else {
                let mut state = controller.read().get_app_state();
                state.write().set_profile_id(&peer_id);
                state.write().set_profile_name(&peer_id);
            }

            if let Ok(loaded_topics) = load_topics_from_file() {
                for topic in loaded_topics {
                    let client_ref = controller.read().get_desktop_client();
                    join_topic_internal(&client_ref, controller.read().get_app_state(), topic)
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to join topic during initialization: {e}");
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
                            let state = controller.read().get_app_state();
                            if let Some(topic_obj) = state().get_topic_mutable(&topic) {
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
                                let mut state = controller.read().get_app_state();
                                if let Some(existing_topic) = state().get_topic(&topic) {
                                    if metadata.timestamp >= existing_topic.last_changed {
                                        state.with_mut(|s| {
                                            s.modify_topic_name(&topic, &metadata.name);
                                            s.modify_topic_avatar(
                                                &topic,
                                                metadata.avatar_url.clone(),
                                            );
                                            s.set_last_changed(&topic, metadata.timestamp);
                                        });
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
                                let state = controller.read().get_app_state();
                                state().get_all_topics().iter().find_map(|topic| {
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
                                let state = controller.read().get_app_state();
                                state().get_topic(&topic).and_then(|topic_obj| {
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
                                })
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

                            let mut state = controller.read().get_app_state();
                            state.with_mut(|s| {
                                if let Some(topic_obj) = s.get_topic_mutable(&topic) {
                                    let message = ui::desktop::models::JoinMessage::new(
                                        join_message.endpoint.to_string(),
                                        Utc::now().timestamp_millis() as u64,
                                    );
                                    topic_obj.add_join_message(message);
                                }
                            });
                        }
                        MessageTypes::LeaveTopic(message) => {
                            let state = controller.read().get_app_state();
                            if let Some(topic_obj) = state().get_topic_mutable(&topic) {
                                let message = ui::desktop::models::LeaveMessage {
                                    sender_id: message.endpoint.to_string(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_leave_message(message);
                            }
                        }
                        MessageTypes::DisconnectTopic(disconnect_msg) => {
                            let mut state = controller.read().get_app_state();
                            state.with_mut(|s| {
                                if let Some(topic_obj) = s.get_topic_mutable(&topic) {
                                    let message = ui::desktop::models::DisconnectMessage {
                                        sender_id: disconnect_msg.endpoint.to_string(),
                                        timestamp: Utc::now().timestamp_millis() as u64,
                                    };

                                    topic_obj.add_disconnect_message(message);
                                }
                            });
                        }
                        MessageTypes::TopicMessages(topic_messages_msg) => {
                            if topic_messages_msg.messages.is_empty() {
                                continue;
                            }

                            let mut state = controller.read().get_app_state();

                            state.with_mut(|s| {
                                if let Some(topic_obj) = s.get_topic_mutable(&topic) {
                                    let received_messages = topic_messages_msg
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
                                }
                            });

                            let missing_messages_opt =
                                state().get_topic(&topic).and_then(|topic_obj| {
                                    let received_messages = topic_messages_msg
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

                                    let missing: Vec<p2p::ChatMessage> = existing_messages
                                        .iter()
                                        .filter(|msg| !received_messages.contains(msg))
                                        .map(|msg| msg.to_message())
                                        .collect();

                                    if !missing.is_empty() {
                                        Some(missing)
                                    } else {
                                        None
                                    }
                                });

                            if let Some(missing_messages) = missing_messages_opt {
                                let client_ref = controller.read().get_desktop_client();
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

                if had_messages
                    && save_topics_to_file(&controller.read().get_app_state()().get_all_topics())
                        .is_err()
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
            let client_ref = controller.read().get_desktop_client();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let client = client_ref.lock().await;
                    let id = client
                        .peer_id()
                        .await
                        .expect("Failed to get peer_id")
                        .parse()
                        .expect("Failed to parse peer_id");

                    let all_topics = controller.read().get_app_state()().get_all_topics();

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
            app_state: controller.read().get_app_state(),
            on_create_topic: move |name: String| {
                controller.read().create_topic(name);
            },
            on_join_topic: move |topic_id: String| {
                controller.read().join_topic(topic_id);
            },
            on_leave_topic: move |topic_id: String| {
                controller.read().leave_topic(topic_id);
            },
            on_send_message: move |(topic_id, message): (String, String)| {
                controller.read().send_message_to_topic(topic_id, message);
            },
            on_modify_topic: move |topic: Topic| {
                controller.read().modify_topic(topic);
            },
            on_modify_profile: move |profile: Profile| {
                controller.read().modify_profile(profile);
            },
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
