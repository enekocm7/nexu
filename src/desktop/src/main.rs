mod client;
mod utils;

use crate::client::DesktopClient;
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
use ui::desktop::models::{AppState, Topic};

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
    app_state: &Mutex<AppState>,
    topic: Topic,
) -> Result<(), Box<dyn Error>> {
    let join_result = desktop_client.lock().await.join_topic(&topic.id).await;
    let mut state = app_state.lock().await;

    match join_result {
        Ok(ticket_str) => {
            state.add_topic(topic);

            if utils::save_topics_to_file(&state.get_all_topics()).is_err() {
                eprintln!("Failed to save topics to file");
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            let ticket = Ticket::from_str(&ticket_str).expect("Invalid ticket string");

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
    let mut app_state = use_signal(|| Mutex::new(AppState::new()));
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
            let writable_ref = app_state.write();
            let mut state = writable_ref.lock().await;
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
            if utils::save_topics_to_file(&state.get_all_topics()).is_err() {
                eprintln!("Failed to save topics to file");
            }
        });
    };

    let on_create_topic = move |name: String| {
        spawn(async move {
            let ticket = desktop_client.read().lock().await.create_topic().await;
            match ticket {
                Ok(ticket) => {
                    let writable_ref = app_state.write();
                    let mut state = writable_ref.lock().await;
                    let topic = Topic::new(ticket.clone(), name, None);
                    state.add_topic(topic);

                    if utils::save_topics_to_file(&state.get_all_topics()).is_err() {
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
            let state = app_state.write();
            if state.lock().await.get_topic(&topic_id).is_some() {
                return;
            }
            let topic = Topic::new_placeholder(topic_id.clone());
            let _ = join_topic_internal(&client_ref, &state, topic).await;
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
                    let writable_ref = app_state.write();
                    let mut state = writable_ref.lock().await;
                    state.remove_topic(&topic_id);

                    if utils::save_topics_to_file(&state.get_all_topics()).is_err() {
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
                    let writable_ref = app_state.write();
                    let mut state = writable_ref.lock().await;
                    if let Some(topic) = state.get_topic(&ticket_id) {
                        let msg = ui::desktop::models::ChatMessage::new(
                            peer_id, ticket_id, message, now, true,
                        );
                        topic.add_message(msg);

                        if utils::save_topics_to_file(&state.get_all_topics()).is_err() {
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

    use_effect(move || {
        let client_ref = desktop_client.read().clone();
        spawn(async move {
            if let Err(e) = client_ref.lock().await.initialize().await {
                eprintln!("Failed to initialize DesktopClient: {}", e);
                return;
            }

            if let Ok(loaded_topics) = utils::load_topics_from_file() {
                for topic in loaded_topics {
                    spawn(async move {
                        let client_ref = desktop_client.read().clone();
                        let state = app_state.write();
                        let _ = join_topic_internal(&client_ref, &state, topic).await;
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
                            let writable_ref = app_state.write();
                            let mut state = writable_ref.lock().await;
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::ChatMessage::new(
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
                                let writable_ref = app_state.write();
                                let mut state = writable_ref.lock().await;
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
                                let readable_ref = app_state.read();
                                let state = readable_ref.lock().await;
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

                            let writable_ref = app_state.write();
                            let mut state = writable_ref.lock().await;
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::JoinMessage {
                                    sender_id: join_message.endpoint.to_string(),
                                    topic_id: topic_obj.id.clone(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_join_message(message);
                            }
                        }
                        MessageTypes::LeaveTopic(message) => {
                            let writable_ref = app_state.write();
                            let mut state = writable_ref.lock().await;
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::LeaveMessage {
                                    sender_id: message.endpoint.to_string(),
                                    topic_id: topic_obj.id.clone(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_leave_message(message);
                            }
                        }
                        MessageTypes::DisconnectTopic(message) => {
                            let writable_ref = app_state.write();
                            let mut state = writable_ref.lock().await;
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let message = ui::desktop::models::DisconnectMessage {
                                    sender_id: message.endpoint.to_string(),
                                    topic_id: topic_obj.id.clone(),
                                    timestamp: Utc::now().timestamp_millis() as u64,
                                };

                                topic_obj.add_disconnect_message(message);
                            }
                        }
                    }
                }

                if had_messages
                    && utils::save_topics_to_file(&app_state.read().lock().await.get_all_topics())
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

                    let state = app_state.read();
                    let all_topics = state.lock().await.get_all_topics();

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
            on_modify_topic
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
