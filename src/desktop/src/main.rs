mod client;
mod controller;
mod message_handler;
mod utils;

use crate::client::DesktopClient;
use crate::utils::contacts::{load_contacts, load_profile};
use crate::utils::topics::{load_topics_from_file, save_topics_to_file};
use chrono::Utc;
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder, use_wry_event_handler};
use dioxus::prelude::*;
use p2p::{MessageTypes, Ticket};
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::desktop_web_components::Desktop;
use ui::desktop::models::{AppState, ImageMessage, Profile, Topic};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    LaunchBuilder::new()
        .with_cfg(
            Config::default().with_menu(None).with_window(
                WindowBuilder::new()
                    .with_title("Nexu")
                    .with_window_icon(load_icon())
                    .with_min_inner_size(LogicalSize::new(1100, 700)),
            ),
        )
        .launch(App);
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
                Ok(id) => id.to_string(),
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

            if let Ok(loaded_contacts) = load_contacts() {
                for contact_chat in loaded_contacts {
                    controller.read().reconnect_to_user(contact_chat);
                }
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
                let state = controller.read().get_app_state();
                message_handler::process_all_messages(&client_ref, state).await;
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
                    let id = client.peer_id().await.expect("Failed to get peer_id");

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
            on_send_message_dm: move |(recipient_id, message): (String, String)| {
                controller.read().send_message_to_user(recipient_id, message);
            },
            on_connect_peer: move |username: String| {
                controller.read().connect_to_user(username);
            },
            on_add_contact: move |username: String| {
                controller.read().connect_to_user(username);
            },
            on_remove_contact: move |profile_id: String| {
                controller.read().remove_contact(profile_id);
            },
            on_image_send: move |(topic_id, image_data): (String, Vec<u8>)| {
                controller.read().send_image_to_topic(topic_id.clone(), image_data);
            }
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

async fn join_topic_internal(
    desktop_client: &Arc<Mutex<DesktopClient>>,
    mut app_state: Signal<AppState>,
    mut topic: Topic,
) -> std::result::Result<(), Box<dyn Error>> {
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

            let id = desktop_client.lock().await.peer_id().await?;

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
