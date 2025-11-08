use dioxus::desktop::{Config, WindowBuilder};
use dioxus::prelude::*;
use ui::desktop::desktop_web_components::Desktop;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    LaunchBuilder::new()
        .with_cfg(
            Config::default()
                .with_menu(None)
                .with_window(WindowBuilder::new().with_title("Nexu")),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Desktop {}
    }
}
