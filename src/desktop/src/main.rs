mod client;
mod controller;
mod media_server;
mod message_handler;
mod utils;

use crate::client::DesktopClient;
use crate::utils::contacts::{load_contacts, load_profile};
use crate::utils::topics::{load_topics_from_file, save_topics_to_file};
use chrono::Utc;
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{use_wry_event_handler, Config, WindowBuilder};
use dioxus::prelude::*;
use p2p::{MessageTypes, Ticket};
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::desktop_web_components::Desktop;
use ui::desktop::models::{AppState, Topic};

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
    let app_state = use_signal(|| AppState::new("Error"));
    let controller = use_signal(controller::AppController::new);
    let mut progress_bar = use_signal::<u64>(|| u64::MAX);

    use_effect(move || {
        let client_ref = controller.read().get_desktop_client();
        let progress_sender = controller.read().progress_bar_sender.clone();
        
        spawn(async move {
            if let Err(e) = client_ref.lock().await.initialize().await {
                eprintln!("Failed to initialize DesktopClient: {}", e);
                return;
            }
            
            spawn(async move {
                controller.read().start_media_server().await;
            });

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
                let mut state = app_state;
                state.write().set_profile_id(&peer_id);
                state.write().set_profile_name(&profile.name);
                state.write().set_profile_avatar(&profile.avatar);
                state.write().set_profile_last_connection_to_now();
            } else {
                let mut state = app_state;
                state.write().set_profile_id(&peer_id);
                state.write().set_profile_name(&peer_id);
            }

            if let Ok(loaded_contacts) = load_contacts() {
                for contact_chat in loaded_contacts {
                    controller
                        .read()
                        .reconnect_to_user_async(app_state, contact_chat)
                        .await;
                }
            }

            if let Ok(loaded_topics) = load_topics_from_file() {
                for topic in loaded_topics {
                    let client_ref = controller.read().get_desktop_client();
                    join_topic_internal(&client_ref, app_state, topic)
                        .await
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to join topic during initialization: {e}");
                        });
                }
            }

            loop {
                message_handler::process_all_messages(&client_ref, app_state).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let command_receiver = controller.read().get_command_receiver();
        let client_for_commands = controller.read().get_desktop_client();
        spawn(async move {
            while let Ok(command) = command_receiver.recv_async().await {
                controller::AppController::process_command(
                    command,
                    app_state,
                    client_for_commands.clone(),
                    progress_sender.clone(),
                )
                .await;
            }
        });

        spawn(async move {
            while let Ok(progress) = controller.read().get_progress_bar().recv_async().await {
                progress_bar.set(progress);
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

                    let all_topics = app_state().get_all_topics();

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
        Desktop::<controller::AppController> { app_state, controller, progress_bar }
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
