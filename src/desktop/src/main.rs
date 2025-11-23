mod client;

use crate::client::DesktopClient;
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::desktop::desktop_web_components::Desktop;
use ui::desktop::models::{AppState, Message, Topic};

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

#[component]
fn App() -> Element {
    let app_state = use_signal(|| AppState::new());
    let desktop_client = use_signal(|| Arc::new(Mutex::new(DesktopClient::new())));

    use_effect(move || {
        let client_ref = desktop_client.read().clone();
        let mut state_clone = app_state.clone();
        spawn(async move {
            if let Err(e) = client_ref.lock().await.initialize().await {
                eprintln!("Failed to initialize DesktopClient: {}", e);
                return;
            }

            loop {
                {
                    let mut client = client_ref.lock().await;
                    let mut state = state_clone.write();
                    for (topic, receiver) in client.get_message_receiver() {
                        while let Ok(message) = receiver.try_recv() {
                            if let Some(topic_obj) = state.get_topic(&topic) {
                                let msg = Message::new(
                                    message.sender.to_string(),
                                    topic.clone(),
                                    message.content,
                                    message.timestamp,
                                    false,
                                );
                                topic_obj.add_message(msg);
                            }
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    });


    let on_create_topic = move |name: String| {
        let mut cloned = app_state.clone();
        let desktop_client_clone = desktop_client.clone();
        spawn(async move {
            let client_ref = desktop_client_clone.read().clone();
            let topic_id_result = client_ref.lock().await.create_topic().await;

            match topic_id_result {
                Ok(topic_id) => {
                    let mut state = cloned.write();
                    let topic = Topic::new(topic_id.clone(), name);
                    state.add_topic(topic);
                }
                Err(e) => eprintln!("Failed to create topic: {}", e),
            }
        });
    };

    let on_join_topic = move |topic_id: String| {
        let mut cloned = app_state.clone();
        let desktop_client_clone = desktop_client.clone();
        spawn(async move {
            let client_ref = desktop_client_clone.read().clone();
            let join_result = client_ref.lock().await.join_topic(&topic_id).await;

            match join_result {
                Ok(ticket_str) => {
                    let mut state = cloned.write();
                    let topic = Topic::new(ticket_str.clone(), ticket_str);
                    state.add_topic(topic);
                }
                Err(e) => eprintln!("Failed to join topic: {}", e),
            }
        });
    };

    let on_send_message = move |(topic_id, message): (String, String)| {
        let mut cloned = app_state.clone();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let desktop_client_clone = desktop_client.clone();
        spawn(async move {
            let client_ref = desktop_client_clone.read().clone();

            let (send_result, peer_id_result) = {
                let client = client_ref.lock().await;
                let send = client.send_message(&topic_id, &message).await;
                let peer = client.peer_id().await;
                (send, peer)
            };

            match (send_result, peer_id_result) {
                (Ok(_), Ok(peer_id)) => {
                    let mut state = cloned.write();
                    if let Some(topic) = state.get_topic(&topic_id) {
                        let msg = Message::new(
                            peer_id,
                            topic_id,
                            message,
                            now,
                            true,
                        );
                        topic.add_message(msg);
                    }
                }
                (Err(e), _) => {
                    eprintln!("Failed to send message to topic {}: {}", topic_id, e);
                }
                (_, Err(e)) => {
                    eprintln!("Failed to get peer_id: {}", e);
                }
            }
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Desktop {
            app_state,
            on_create_topic,
            on_join_topic,
            on_send_message
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
