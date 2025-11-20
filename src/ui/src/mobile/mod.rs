#[cfg(feature = "mobile")]
pub mod mobile_components {
    use dioxus::prelude::*;

    static MAIN_CSS: Asset = asset!("/assets/styling/mobile.css");

    #[component]
    pub fn Mobile() -> Element {
        rsx! {
            link { rel: "stylesheet", href: MAIN_CSS }
            title { "Nexu Mobile" }
            body { class: "mobile-body",
                h1 { class: "mobile-header", "Mobile UI Component" }
            }
        }
    }
}
