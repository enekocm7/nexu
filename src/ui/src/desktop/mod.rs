mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {

    use chrono::{DateTime, TimeDelta, Utc};
    use dioxus::prelude::*;

    use crate::desktop::models::{Topic, TopicCreationMode};

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");
    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");

    #[component]
    pub fn Desktop() -> Element {
        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            div { class: "desktop-body",
                Column {
                    contacts: vec![
                        Topic {
                            id: "1".to_string(),
                            name: "Alice".to_string(),
                            avatar_url: None,
                            last_connection: Some(1625155200),
                            last_message: Some("Hey, how are you?".to_string()),
                        },
                        Topic {
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
    fn Column(contacts: Vec<Topic>) -> Element {
        let mut show_topic_dialog = use_signal(|| false);

        rsx! {
            div { class: "desktop-column",
                div { class: "desktop-column-header",
                    div { class: "desktop-column-top-bar",
                        h2 { class: "desktop-column-title", "Messages" }
                        button {
                            class: "desktop-column-new-message-button",
                            title: "New Topic",
                            onclick: move |_| show_topic_dialog.set(true),
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

                if show_topic_dialog() {
                    TopicDialog { toggle: show_topic_dialog }
                }
            }
        }
    }

    #[component]
    fn TopicDialog(mut toggle: Signal<bool>) -> Element {
        let mut topic_name = use_signal(String::new);
        let mut selected_mode = use_signal(|| TopicCreationMode::Create);

        rsx! {
            div { class: "topic-dialog-overlay",
                onclick: move |_| {
                    toggle.set(false);
                    topic_name.set(String::new());
                },
                
                div { class: "topic-dialog",
                    onclick: move |e| {
                        e.stop_propagation();
                    },
                    div { class: "topic-dialog-header",
                        h3 { class: "topic-dialog-title", "New Topic" }
                        button {
                            class: "topic-dialog-close",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            img { 
                                src: CLOSE_ICON
                            }
                        }
                    }
                    div { class: "topic-dialog-body",
                        div { class: "topic-mode-tabs",
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Create { "topic-mode-tab active" } else { "topic-mode-tab" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Create),
                                "Create Topic"
                            }
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Join { "topic-mode-tab active" } else { "topic-mode-tab" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Join),
                                "Join Topic"
                            }
                        }
                        div { class: "topic-input-group",
                            label { class: "topic-input-label",
                                if *selected_mode.read() == TopicCreationMode::Create {
                                    "Topic Name"
                                } else {
                                    "Topic ID or Invite Link"
                                }
                            }
                            input {
                                class: "topic-input",
                                r#type: "text",
                                value: "{topic_name}",
                                placeholder: if *selected_mode.read() == TopicCreationMode::Create { "Enter topic name..." } else { "Enter topic ID or paste invite link..." },
                                oninput: move |e| topic_name.set(e.value()),
                            }
                        }
                        p { class: "topic-dialog-description",
                            if *selected_mode.read() == TopicCreationMode::Create {
                                "Create a new topic to start chatting with others. You can share the topic ID with your friends."
                            } else {
                                "Join an existing topic by entering its ID or invite link shared by a friend."
                            }
                        }
                    }
                    div { class: "topic-dialog-footer",
                        button {
                            class: "topic-dialog-button topic-dialog-button-cancel",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            "Cancel"
                        }
                        button { class: "topic-dialog-button topic-dialog-button-primary",
                            disabled: topic_name().trim().is_empty(),
                            onclick: move |_| {
                                //TODO Handle topic creation
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            if *selected_mode.read() == TopicCreationMode::Create {
                                "Create"
                            } else {
                                "Join"
                            }
                        }
                    }
                }
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
    fn ContactItem(contact: Topic) -> Element {
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
