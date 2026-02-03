#[cfg(feature = "mobile")]
pub mod mobile_components {
    use dioxus::prelude::*;

    #[allow(clippy::volatile_composites)]
    #[component]
    pub fn Mobile() -> Element {
        rsx! {
            link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
            title { "Nexu Mobile" }
            body { class: "mobile-body",
                h1 { class: "mobile-header", "Mobile UI Component" }
            }
        }
    }
}
