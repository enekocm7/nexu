mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::desktop::models::Contact;
    use dioxus::prelude::*;

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");

    #[component]
    pub fn Desktop() -> Element {
        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            title { "Nexu Desktop" }
            body { class: "desktop-body",
                Column {
                    contacts: vec![
                        Contact {
                            id: "1".to_string(),
                            name: "Alice".to_string(),
                            avatar_url: None,
                            last_connection: Some(1625155200),
                            last_message: Some("Hey, how are you?".to_string()),
                        },
                        Contact {
                            id: "2".to_string(),
                            name: "Bob".to_string(),
                            avatar_url: None,
                            last_connection: Some(1625241600),
                            last_message: Some("Let's catch up later.".to_string()),
                        },
                    ]
                }
            }
        }
    }

    #[component]
    fn Column(contacts: Vec<Contact>) -> Element {
        rsx! {
            div { class: "desktop-column",
                div { class: "desktop-column-header",
                    div { class: "desktop-column-top-bar",
                        h2 { class: "desktop-column-title",
                             "Messages"
                        }
                        button {
                            class: "desktop-column-new-message-button",
                            icon: "plus",
                            title: "New Chat"
                        }
                    }
                    input {
                        class: "desktop-column-search",
                        r#type: "text",
                        icon: "search",
                        placeholder: "Search"
                    }
                }
                div { class: "desktop-column-contacts",
                    ul {
                        for contact in contacts.iter() {
                            ContactItem { contact: contact.clone() }
                        }
                    }
                }
                div { class: "desktop-column-footer"

                }
            }
        }
    }

    #[component]
    fn ContactItem(contact: Contact) -> Element {
        rsx! {
            div { class: "desktop-contact-item",
                img { class: "desktop-contact-avatar", src: "{contact.avatar_url.clone().unwrap_or_default()}" }
                div { class: "desktop-contact-info",
                    h3 { class: "desktop-contact-name", "{contact.name}" }
                    p { class: "desktop-contact-last-message", "{contact.last_message.clone().unwrap_or_default()}" }
                }
                h3 {class: "desktop-contact-last-connection",
                    "{contact.last_connection.unwrap_or_default()}"
                }
            }
        }
    }
}
