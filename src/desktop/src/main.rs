mod client;

use crate::client::DesktopClient;
use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use std::sync::Arc;
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
    let desktop_client = use_signal(|| Arc::new(DesktopClient::new()));

    use_effect(move || {
        let client_ref = desktop_client.read().clone();
        spawn(async move {
            if let Err(e) = client_ref.initialize().await {
                eprintln!("Failed to initialize DesktopClient: {}", e);
            }
        });
    });

    let on_create_topic = move |name: String| {
        let mut cloned = app_state.clone();
        let client_ref = desktop_client.read().clone();
        spawn(async move {
            match client_ref.create_topic().await {
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
        let client_ref = desktop_client.read().clone();
        spawn(async move {
            match client_ref.join_topic(&topic_id).await {
                Ok(_) => {
                    let mut state = cloned.write();
                    let topic = Topic::new(topic_id.clone(), topic_id.to_string());
                    state.add_topic(topic);
                }
                Err(e) => eprintln!("Failed to join topic: {}", e),
            }
        });
    };

    let on_send_message = move |(topic_id, message): (String, String)| {
        let mut cloned = app_state.clone();
        let client_ref = desktop_client.read().clone();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        spawn(async move {
            match client_ref.send_message(&topic_id, &message).await {
                Ok(_) => {
                    let mut state = cloned.write();
                    if let Some(topic) = state.get_topic(&topic_id) {
                        let message = Message::new(
                            client_ref.peer_id().await.unwrap(),
                            topic_id,
                            message,
                            now,
                            true,
                        );
                        topic.add_message(message);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to send message to topic {}: {}", topic_id, e);
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
            on_send_message,
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
