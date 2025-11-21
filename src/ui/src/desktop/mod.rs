pub mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use chrono::{DateTime, TimeDelta, Utc};
    use dioxus::prelude::*;

    use crate::desktop::models::{AppState, Message, Topic, TopicCreationMode};

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");
    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");

    #[component]
    pub fn Desktop(
        app_state: Signal<AppState>,
        on_create_topic: EventHandler<String>,
        on_join_topic: EventHandler<String>,
        on_send_message: EventHandler<(String, String)>,
    ) -> Element {
        let contacts = app_state
            .read()
            .get_all_topics()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        let mut show_topic_dialog = use_signal(|| false);
        let selected_topic = use_signal::<Option<String>>(|| None);

        let mut search_query = use_signal(String::new);

        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            div { class: "desktop-body",
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
                            oninput: move |value| {
                                search_query.set(value.value());
                            },
                        }
                    }
                    div { class: "desktop-column-contacts",
                        ul {
                            for contact in contacts
                                .iter()
                                .filter(|contact| {
                                    contact.name.to_lowercase().contains(&search_query().to_lowercase())
                                })
                            {
                                ContactItem {
                                    contact: contact.clone(),
                                    on_select: selected_topic,
                                }
                            }
                        }
                    }

                    if show_topic_dialog() {
                        TopicDialog {
                            toggle: show_topic_dialog,
                            on_create: on_create_topic,
                            on_join: on_join_topic,
                        }
                    }
                }
                if let Some(topic_id) = selected_topic() {
                    Chat { app_state: app_state.clone(), topic_id: topic_id.clone(), on_send_message }
                } else {
                    div { class: "desktop-chat-placeholder",
                        h2 { "Select a topic to start chatting" }
                    }
                }
            }
        }
    }

    #[component]
    fn TopicDialog(
        mut toggle: Signal<bool>,
        on_create: EventHandler<String>,
        on_join: EventHandler<String>,
    ) -> Element {
        let mut topic_name = use_signal(String::new);
        let mut selected_mode = use_signal(|| TopicCreationMode::Create);

        let handle_submit = move |_| {
            let mode = *selected_mode.read();
            let name = topic_name().trim().to_string();

            if !name.is_empty() {
                match mode {
                    TopicCreationMode::Create => on_create.call(name),
                    TopicCreationMode::Join => on_join.call(name),
                }
                toggle.set(false);
                topic_name.set(String::new());
            }
        };

        rsx! {
            div {
                class: "topic-dialog-overlay",
                onclick: move |_| {
                    toggle.set(false);
                    topic_name.set(String::new());
                },
                div {
                    class: "topic-dialog",
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
                            img { src: CLOSE_ICON }
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
                        button {
                            class: "topic-dialog-button topic-dialog-button-primary",
                            disabled: topic_name().trim().is_empty(),
                            onclick: handle_submit,
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

    #[component]
    fn ContactItem(contact: Topic, on_select: Signal<Option<String>>) -> Element {
        let avatar_url = if let Some(url) = &contact.avatar_url {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let time_display = if let Some(timestamp) = contact.last_connection {
            format_relative_time(timestamp as i64)
        } else {
            String::from("")
        };

        rsx! {
            div {
                class: "desktop-contact-item",
                onclick: move |_| {
                    on_select.set(Some(contact.id.clone()));
                },
                img { class: "desktop-contact-avatar", src: "{avatar_url}", alt: "{contact.name}", draggable: "false" }
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

    #[component]
    fn Chat(
        app_state: Signal<AppState>,
        topic_id: String,
        on_send_message: EventHandler<(String, String)>,
    ) -> Element {
        let topic_opt = app_state.read().get_topic_immutable(&topic_id).cloned();

        if let Some(topic_read) = topic_opt {
            let messages = topic_read.messages.clone();
            let topic_name = topic_read.name.clone();

            let avatar_url = if let Some(url) = &topic_read.avatar_url {
                url.clone()
            } else {
                DEFAULT_AVATAR.to_string()
            };

            let mut message_input = use_signal(String::new);

            let mut handle_send_message = {
                let topic_id = topic_id.clone();
                let on_send_message = on_send_message.clone();
                move |_| {
                    let content = message_input().trim().to_string();
                    if !content.is_empty() {
                        on_send_message.call((topic_id.clone(), content));
                        message_input.set(String::new());
                    }
                }
            };

            rsx! {
                div { class: "desktop-chat-window",
                    div { class: "desktop-chat-header",
                        img { class: "desktop-contact-avatar", src: "{avatar_url}" }
                        h2 { class: "desktop-contact-name", "{topic_name}" }
                    }
                    div { class: "desktop-chat-messages",
                        for message in messages.iter() {
                            ChatMessage { message: message.clone() }
                        }
                    }
                    div { class: "desktop-chat-input-area",
                        input {
                            class: "desktop-chat-input",
                            r#type: "text",
                            placeholder: "Type a message...",
                            value: "{message_input()}",
                            oninput: move |e| {
                                message_input.set(e.value());
                            },
                        }
                        button {
                            class: "desktop-chat-send-button",
                            onclick: move |_| {
                                handle_send_message(());
                                message_input.set(String::new())
                            },
                            "Send"
                        }
                    }
                }
            }
        } else {
            rsx! {
                div { class: "desktop-chat-placeholder",
                    h2 { "Topic not found" }
                }
            }
        }
    }

    #[component]
    fn ChatMessage(message: Message) -> Element {
        rsx! {
            div { class: if message.is_sent { "chat-message sent" } else { "chat-message received" },
                p { class: "message-text", "{message.content}" }
                p { class: "chat-message-timestamp", "10:30 AM" }
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
}
