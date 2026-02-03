#![allow(clippy::volatile_composites)]

use dioxus::prelude::*;
use ui::mobile::mobile_components::Mobile;

const FAVICON: Asset = asset!("/assets/favicon.ico");

fn main() {
    launch(App);
}

fn get_device_type() -> DeviceType {
    let window = web_sys::window().expect("Error getting window");
    let navigator = window.navigator();
    if navigator.max_touch_points() > 0 {
        DeviceType::Mobile
    } else {
        DeviceType::Desktop
    }
}

#[component]
fn App() -> Element {
    let device_type = use_signal_sync(get_device_type);

    rsx! {
        head {
            document::Link { rel: "icon", href: FAVICON }
            Title { "Nexu" }
        }
        match *device_type.read() {
            DeviceType::Mobile => rsx! {
                Mobile {}
            },
            //TODO Desktop p2p implementation
            DeviceType::Desktop => rsx! {},
        }
    }
}

enum DeviceType {
    Mobile,
    Desktop,
}
