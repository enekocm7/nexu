pub mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::components::toast::ToastProvider;
    use crate::desktop::models::{AppState, Message, Topic, TopicCreationMode};
    use arboard::Clipboard;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use chrono::{DateTime, Local, TimeDelta};
    use dioxus::prelude::*;
    use dioxus_primitives::context_menu::{
        ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
    };
    use dioxus_primitives::toast::{ToastOptions, use_toast};
    use tokio::sync::Mutex;

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");
    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");
    static COMPONENTS_CSS: Asset = asset!("/assets/dx-components-theme.css");

    #[component]
    pub fn Desktop(
        app_state: Signal<Mutex<AppState>>,
        on_create_topic: EventHandler<String>,
        on_join_topic: EventHandler<String>,
        on_leave_topic: EventHandler<String>,
        on_modify_topic: EventHandler<Topic>,
        on_send_message: EventHandler<(String, String)>,
    ) -> Element {
        let mut show_topic_dialog = use_signal(|| false);
        let mut selected_topic = use_signal::<Option<String>>(|| None);
        let mut show_topic_details = use_signal::<Option<Topic>>(|| None);
        let mut search_query = use_signal(String::new);
        let mut show_leave_confirmation = use_signal::<Option<(String, String)>>(|| None);

        rsx! {
            link { rel: "stylesheet", href: DESKTOP_CSS }
            link { rel: "stylesheet", href: COMPONENTS_CSS }
            div {
                class: "desktop-body",
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
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
                            }
                        }
                    }
                    div { class: "desktop-column-contacts",
                        {
                            let contacts_resource = use_resource(move || async move {
                                let mut contacts = app_state
                                    .read()
                                    .lock()
                                    .await
                                    .get_all_topics()
                                    .into_iter()
                                    .collect::<Vec<Topic>>();
                                contacts.sort_by(|a, b| b.last_connection.cmp(&a.last_connection));
                                contacts
                            });

                            match &*contacts_resource.read_unchecked() {
                                Some(contacts) => rsx! {
                                    ul {
                                        {
                                            contacts
                                                .iter()
                                                .filter(|contact| {
                                                    contact.name.to_lowercase().contains(&search_query().to_lowercase())
                                                })
                                                .map(|contact| {
                                                    let contact_id = contact.id.clone();
                                                    let contact_name = contact.name.clone();
                                                    let contact_for_details = contact.clone();
                                                    let contact_clone = contact.clone();
                                                    rsx!{
                                                        ContextMenu{
                                                            ContextMenuTrigger {
                                                                TopicItem { contact: Signal::new(contact_clone), on_select: selected_topic }
                                                            }
                                                            ContextMenuContent { class: "context-menu-content",
                                                                ContextMenuItem { class: "context-menu-item",
                                                                    value: "Open Chat".to_string(),
                                                                    index: 0usize,
                                                                    on_select: {
                                                                        let contact_id = contact_id.clone();
                                                                        move |_| {
                                                                            selected_topic.set(Some(contact_id.clone()));
                                                                        }
                                                                    },
                                                                    "Open Chat"
                                                                }
                                                                ContextMenuItem { class: "context-menu-item",
                                                                    value: "Open Details".to_string(),
                                                                    index: 1usize,
                                                                    on_select:  move |_| {
                                                                        show_topic_details.set(Some(contact_for_details.clone()));
                                                                    },
                                                                    "Open Details"
                                                                }
                                                                ContextMenuItem { class: "context-menu-item context-menu-item-danger",
                                                                    value: "Leave Topic".to_string(),
                                                                    index: 2usize,
                                                                    on_select:  {
                                                                        let contact_id = contact_id.clone();
                                                                        let contact_name = contact_name.clone();
                                                                        move |_| {
                                                                            show_leave_confirmation.set(Some((contact_id.clone(), contact_name.clone())))
                                                                        }
                                                                    },
                                                                    "Leave Topic"
                                                                }
                                                            }
                                                        }
                                                    }
                                                })
                                        }
                                    }
                                },
                                None => rsx! { div { "Loading..." } }
                            }
                        }
                    }

                    if let Some(topic) = show_topic_details() {
                        ToastProvider {
                            TopicDetails { topic: topic.clone(), toggle: show_topic_details, on_modify_topic }
                        }
                    }

                    if show_topic_dialog() {
                        TopicDialog { toggle: show_topic_dialog, on_create: on_create_topic, on_join: on_join_topic }
                    }

                    if let Some((topic_id, topic_name)) = show_leave_confirmation() {
                        ConfirmationDialog {
                            title: "Leave Topic".to_string(),
                            message: format!(
                                "Are you sure you want to leave \"{}\"? You will no longer receive messages from this topic.",
                                topic_name,
                            ),
                            confirm_text: "Leave".to_string(),
                            cancel_text: "Cancel".to_string(),
                            is_danger: true,
                            toggle: show_leave_confirmation,
                            on_confirm: move |_| {
                                on_leave_topic.call(topic_id.clone());
                                show_leave_confirmation.set(None);
                                selected_topic.set(None);
                            }
                        }
                    }
                }
                if let Some(topic_id) = selected_topic() {
                    Chat { app_state, topic_id: topic_id.clone(), on_send_message }
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
                                class: if *selected_mode.read() == TopicCreationMode::Create {
                                    "topic-mode-tab active"
                                } else {
                                    "topic-mode-tab"
                                },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Create),
                                "Create Topic"
                            }
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Join {
                                    "topic-mode-tab active"
                                } else {
                                    "topic-mode-tab"
                                },
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
                                placeholder: if *selected_mode.read() == TopicCreationMode::Create {
                                    "Enter topic name..."
                                } else {
                                    "Enter topic ID or paste invite link..."
                                },
                                oninput: move |e| topic_name.set(e.value())
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
    fn ConfirmationDialog(
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
        is_danger: bool,
        mut toggle: Signal<Option<(String, String)>>,
        on_confirm: EventHandler<()>,
    ) -> Element {
        let button_class = if is_danger {
            "confirmation-dialog-button confirmation-dialog-button-danger"
        } else {
            "confirmation-dialog-button confirmation-dialog-button-primary"
        };

        rsx! {
            div {
                class: "confirmation-dialog-overlay",
                onclick: move |_| toggle.set(None),
                div {
                    class: "confirmation-dialog",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "confirmation-dialog-header",
                        h3 { class: "confirmation-dialog-title", "{title}" }
                        button {
                            class: "confirmation-dialog-close",
                            onclick: move |_| toggle.set(None),
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "confirmation-dialog-body",
                        p { class: "confirmation-dialog-message", "{message}" }
                    }
                    div { class: "confirmation-dialog-footer",
                        button {
                            class: "confirmation-dialog-button confirmation-dialog-button-cancel",
                            onclick: move |_| toggle.set(None),
                            "{cancel_text}"
                        }
                        button {
                            class: "{button_class}",
                            onclick: move |_| on_confirm.call(()),
                            "{confirm_text}"
                        }
                    }
                }
            }
        }
    }

    #[component]
    fn TopicItem(contact: Signal<Topic>, on_select: Signal<Option<String>>) -> Element {
        let topic = contact.read().clone();
        let topic_id = topic.id.clone();
        let topic_name = topic.name.clone();
        let last_message = topic.last_message.clone().unwrap_or_default();

        let avatar_url = if let Some(url) = &topic.avatar_url
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let time_display = if let Some(timestamp) = topic.last_connection {
            format_relative_time(timestamp as i64)
        } else {
            String::from("")
        };

        rsx! {
            div {
                class: "desktop-contact-item",
                onclick: move |_| {
                    on_select.set(Some(topic_id.clone()));
                },
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                img {
                    class: "desktop-contact-avatar",
                    src: "{avatar_url}",
                    alt: "{topic_name}",
                    draggable: "false"
                }
                div { class: "desktop-contact-info",
                    h3 { class: "desktop-contact-name", "{topic_name}" }
                    p { class: "desktop-contact-last-message", "{last_message}" }
                }
                h3 { class: "desktop-contact-last-connection", "{time_display}" }
            }
        }
    }

    #[component]
    fn Chat(
        app_state: Signal<Mutex<AppState>>,
        topic_id: String,
        on_send_message: EventHandler<(String, String)>,
    ) -> Element {
        let topic_id_clone = topic_id.clone();
        let topic_opt = use_resource(move || {
            let tid = topic_id_clone.clone();
            async move {
                app_state
                    .read()
                    .lock()
                    .await
                    .get_topic_immutable(&tid)
                    .cloned()
            }
        });

        match &*topic_opt.read_unchecked() {
            Some(Some(topic_read)) => {
                let messages = topic_read.messages.clone();
                let topic_name = topic_read.name.clone();

                let avatar_url = if let Some(url) = &topic_read.avatar_url {
                    url.clone()
                } else {
                    DEFAULT_AVATAR.to_string()
                };

                let mut message_input = use_signal(String::new);

                let send_message = use_callback({
                    let topic_id = topic_id.clone();
                    move |_| {
                        let content = message_input().trim().to_string();
                        if !content.is_empty() {
                            on_send_message.call((topic_id.clone(), content));
                            message_input.set(String::new());
                        }
                    }
                });

                rsx! {
                    div { class: "desktop-chat-window",
                        div { class: "desktop-chat-header",
                            img {
                                class: "desktop-contact-avatar",
                                src: "{avatar_url}"
                            }
                            h2 {
                                class: "desktop-contact-name",
                                title: "{topic_name}",
                                "{topic_name}"
                            }
                        }
                        div { class: "desktop-chat-messages",
                            for message in messages.iter() {
                                ChatMessageComponent { message: message.clone() }
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
                                onkeypress: move |e| {
                                    if e.key() == Key::Enter {
                                        send_message(());
                                    }
                                }
                            }
                            button {
                                class: "desktop-chat-send-button",
                                onclick: move |_| {
                                    send_message(());
                                },
                                "Send"
                            }
                        }
                    }
                }
            }
            _ => {
                rsx! {
                    div { class: "desktop-chat-placeholder",
                        h2 { "Loading..." }
                    }
                }
            }
        }
    }

    #[component]
    fn ChatMessageComponent(message: Message) -> Element {
        match message {
            Message::Chat(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                rsx! {
                    div { class: if message.is_sent { "chat-message sent" } else { "chat-message received" },
                        p { class: "message-sender-id", "{message.sender_id}" }
                        p { class: "message-text", "{message.content}" }
                        p { class: "chat-message-timestamp", "{timestamp_str}" }
                    }
                }
            }
            Message::Leave(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                rsx! {
                    div { class: "chat-message system-message",
                        p { class: "message-text", "{message.sender_id} has left the topic." }
                        p { class: "system-message-timestamp", "{timestamp_str}" }
                    }
                }
            }
            Message::Join(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                let sender = message.sender_id.clone();
                let text = if message.me {
                    format!("{sender} joined the topic.")
                } else {
                    format!("{sender} has joined the topic.")
                };

                rsx! {
                    div { class: "chat-message system-message",
                        p { class: "message-text", "{text}" }
                        p { class: "system-message-timestamp", "{timestamp_str}" }
                    }
                }
            }
            Message::Disconnect(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                rsx! {
                    div { class: "chat-message system-message",
                        p { class: "message-text", "{message.sender_id} has disconnected." }
                        p { class: "system-message-timestamp", "{timestamp_str}" }
                    }
                }
            }
        }
    }

    #[component]
    fn TopicDetails(
        topic: Topic,
        mut toggle: Signal<Option<Topic>>,
        on_modify_topic: EventHandler<Topic>,
    ) -> Element {
        let toast = use_toast();
        let mut edited_title = use_signal(|| topic.name.clone());

        let handle_copy_topic_id = {
            let topic_id = topic.id.clone();
            move |_event: Event<MouseData>| match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(topic_id.clone()) {
                    Ok(_) => {
                        toast.success(
                            "Topic ID copied to clipboard!".to_owned(),
                            ToastOptions::default(),
                        );
                    }
                    Err(_) => {
                        toast.error(
                            "Error copying Topic ID.".to_owned(),
                            ToastOptions::default(),
                        );
                    }
                },
                Err(_) => {
                    toast.error(
                        "Error accessing clipboard.".to_owned(),
                        ToastOptions::default(),
                    );
                }
            }
        };

        let topic_clone = topic.clone();
        let handle_save = move |_event: Event<MouseData>| {
            let mut updated_topic = topic_clone.clone();
            updated_topic.name = edited_title().trim().to_string();
            on_modify_topic.call(updated_topic);
            toast.success(
                "Topic updated successfully".to_owned(),
                ToastOptions::default(),
            );
            toggle.set(None);
        };

        let topic_clone_for_image = topic.clone();
        let handle_image_change = move |event: Event<FormData>| {
            let files = event.files();
            if let Some(file) = files.first() {
                let file = file.clone();
                let topic_clone = topic_clone_for_image.clone();
                spawn(async move {
                    match file.read_bytes().await {
                        Ok(bytes) => {
                            const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                            if bytes.len() > MAX_SIZE {
                                toast.error(
                                    "Image size must be less than 512 KB".to_owned(),
                                    ToastOptions::default(),
                                );
                                return;
                            }

                            let base64 = BASE64_STANDARD.encode(&bytes);
                            let url =
                                format!("data:{};base64,{}", file.content_type().unwrap(), base64);

                            let mut updated_topic = topic_clone.clone();
                            updated_topic.avatar_url = Some(url);
                            on_modify_topic.call(updated_topic);
                            toggle.set(None);

                            toast.success(
                                "Topic avatar updated successfully".to_owned(),
                                ToastOptions::default(),
                            );
                        }
                        Err(e) => {
                            toast.error(
                                format!("Failed to read file: {}", e),
                                ToastOptions::default(),
                            );
                        }
                    }
                });
            } else {
                toast.error("No file selected.".to_owned(), ToastOptions::default());
            }
        };

        let avatar_url = if let Some(url) = &topic.avatar_url
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        rsx! {
            div {
                class: "topic-details-overlay",
                onclick: move |_| toggle.set(None),
                div {
                    class: "topic-details",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "topic-details-header",
                        label { class: "topic-details-image-wrapper",
                            img { class: "topic-details-image", src: avatar_url }
                            input {
                                r#type: "file",
                                style: "display: none;",
                                onchange: handle_image_change
                            }
                        }
                        input {
                            class: "topic-details-title",
                            r#type: "text",
                            value: "{edited_title}",
                            oninput: move |e| edited_title.set(e.value())
                        }
                        button {
                            class: "topic-details-save-button",
                            onclick: handle_save,
                            "Save"
                        }
                    }
                    hr {}
                    p { class: "topic-details-section-title", "Topic ID" }
                    p {
                        class: "topic-details-topic-id",
                        title: "Click to copy",
                        onclick: handle_copy_topic_id,
                        "{topic.id}"
                    }
                }
            }
        }
    }

    fn format_message_timestamp(timestamp: u64) -> String {
        let timestamp_secs = (timestamp / 1000) as i64;
        let datetime = match DateTime::from_timestamp(timestamp_secs, 0) {
            Some(dt) => dt.with_timezone(&Local),
            None => return String::from(""),
        };

        let now = Local::now();
        let duration = now.signed_duration_since(datetime);

        if duration < TimeDelta::days(1) {
            return datetime.format("%I:%M %p").to_string();
        }

        if duration < TimeDelta::days(2) {
            return format!("Yesterday {}", datetime.format("%I:%M %p"));
        }

        if duration < TimeDelta::weeks(1) {
            return datetime.format("%a %I:%M %p").to_string();
        }

        datetime.format("%m/%d/%y %I:%M %p").to_string()
    }

    fn format_relative_time(timestamp: i64) -> String {
        let last_connection = match DateTime::from_timestamp(timestamp, 0) {
            Some(dt) => dt.with_timezone(&Local),
            None => return String::from(""),
        };

        let now = Local::now();
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
