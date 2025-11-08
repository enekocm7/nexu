#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use dioxus::prelude::*;

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");

    #[component]
    pub fn Desktop() -> Element {
        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            title { "Nexu Desktop" }
            body { class: "desktop-body",
                h1 { class: "desktop-header",
                    "Desktop UI Component"
                }
            }
        }
    }
}
