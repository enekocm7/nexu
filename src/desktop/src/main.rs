mod topic_service;
mod client;

use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use ui::desktop::desktop_web_components::Desktop;
use ui::desktop::models::AppState;

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

    let on_create_topic = move |name: String| {
        let mut cloned = app_state.clone();
        spawn(async move {
            let mut state = cloned.write();
            topic_service::create_topic(name, &mut state).await;
        });
    };

    let on_join_topic = move |topic_id: String| {
        let mut cloned = app_state.clone();
        spawn(async move {
            let mut state = cloned.write();
            topic_service::join_topic(topic_id, &mut state).await;
        });
    };

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Desktop { app_state, on_create_topic, on_join_topic }
    }
}

fn load_icon() -> Option<Icon> {
    let icon_bytes = include_bytes!("../assets/logo.png");
    let img = image::load_from_memory(icon_bytes).expect("Failed to load icon image");

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}
