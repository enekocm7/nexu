use dioxus::prelude::*;
use ui::mobile::mobile_components::Mobile;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Mobile {

        }

    }
}
