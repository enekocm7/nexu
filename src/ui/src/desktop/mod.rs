mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    
    use chrono::{DateTime, TimeDelta, Utc};
    use dioxus::prelude::*;

    use crate::desktop::models::Contact;

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");
    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");

    #[component]
    pub fn Desktop() -> Element {
        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            div { class: "desktop-body",
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
                    ],
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
                        h2 { class: "desktop-column-title", "Messages" }
                        button {
                            class: "desktop-column-new-message-button",
                            title: "New Chat",
                            "+"
                        }
                    }
                    input {
                        class: "desktop-column-search",
                        r#type: "text",
                        icon: "search",
                        placeholder: "Search",
                    }
                }
                div { class: "desktop-column-contacts",
                    ul {
                        for contact in contacts.iter() {
                            ContactItem { contact: contact.clone() }
                        }
                    }
                }
                div { class: "desktop-column-footer" }
            }
        }
    }

    fn format_relative_time(timestamp: i64) -> String {
        let last_connection = match DateTime::from_timestamp(timestamp, 0) {
            Some(dt) => dt,
            None => return String::from(""),
        };

        let now = Utc::now();
        let duration = now.signed_duration_since(last_connection);

        if duration < TimeDelta::minutes(1) {
            return String::from("Just now");
        }

        if duration < TimeDelta::hours(1) {
            let minutes = duration.num_minutes();
            return format!("{}m ago", minutes);
        }

        if duration < TimeDelta::days(1) {
            let hours = duration.num_hours();
            return format!("{}h ago", hours);
        }

        if duration < TimeDelta::days(2) {
            return String::from("Yesterday");
        }

        if duration < TimeDelta::weeks(1) {
            let days = duration.num_days();
            return format!("{} days ago", days);
        }

        last_connection.format("%m/%d/%Y").to_string()
    }

    #[component]
    fn ContactItem(contact: Contact) -> Element {
        let avatar_url = if contact.avatar_url.is_some() {
            contact.avatar_url.unwrap()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let time_display = if let Some(timestamp) = contact.last_connection {
            format_relative_time(timestamp as i64)
        } else {
            String::from("")
        };

        rsx! {
            div { class: "desktop-contact-item",
                img { class: "desktop-contact-avatar", src: "{avatar_url}" }
                div { class: "desktop-contact-info",
                    h3 { class: "desktop-contact-name", "{contact.name}" }
                    p { class: "desktop-contact-last-message",
                        "{contact.last_message.clone().unwrap_or_default()}"
                    }
                }
                h3 { class: "desktop-contact-last-connection", "{time_display}" }
            }
        }
    }
}
