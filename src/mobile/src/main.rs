use dioxus::prelude::*;
use ui::mobile::mobile_components::Mobile;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Mobile {
        }

    }
}
