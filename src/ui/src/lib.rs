//! This crate contains all shared UI for the workspace.

use dioxus::prelude::*;

pub mod desktop;

static MAIN_CSS: Asset = asset!("/assets/styling/mobile.css");

#[component]
pub fn Mobile() -> Element {
    rsx! {
        link { rel: "stylesheet", href: MAIN_CSS }
        title { "Nexu Mobile"}
        body { class: "mobile-body",
            h1 { class: "mobile-header",
                "Mobile UI Component"
            }
        }
    }
}
