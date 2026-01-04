pub mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::components::toast::ToastProvider;
    use crate::desktop::models::{AppState, Message, Profile, Topic, TopicCreationMode};
    use arboard::Clipboard;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use chrono::{DateTime, Local, TimeDelta};
    use dioxus::prelude::*;
    use dioxus_primitives::context_menu::{
        ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
    };
    use dioxus_primitives::toast::{ToastOptions, use_toast};

    static DESKTOP_CSS: Asset = asset!("/assets/styling/desktop.css");
    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");
    static COMPONENTS_CSS: Asset = asset!("/assets/dx-components-theme.css");

    #[component]
    pub fn Desktop(
        app_state: Signal<AppState>,
        on_create_topic: EventHandler<String>,
        on_join_topic: EventHandler<String>,
        on_leave_topic: EventHandler<String>,
        on_modify_topic: EventHandler<Topic>,
        on_send_message: EventHandler<(String, String)>,
        on_modify_profile: EventHandler<Profile>,
    ) -> Element {
        let mut show_topic_dialog = use_signal(|| false);
        let mut selected_topic_id = use_signal::<Option<String>>(|| None);
        let mut show_topic_details = use_signal::<Option<Topic>>(|| None);
        let mut show_profile_details = use_signal::<Option<Profile>>(|| None);
        let mut search_query = use_signal(String::new);
        let mut show_leave_confirmation = use_signal::<Option<(String, String)>>(|| None);

        let mut contacts_list: Vec<Topic> = {
            let state = app_state.read();
            let mut contacts = state.get_all_topics().into_iter().collect::<Vec<Topic>>();
            contacts.sort_by(|a, b| b.last_connection.cmp(&a.last_connection));
            contacts
        };

        let profile_data: Profile = {
            let state = app_state.read();
            state.get_profile()
        };

        let avatar_url = if let Some(url) = &profile_data.avatar
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let profile_for_click = profile_data.clone();

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
                            },
                        }
                    }
                    div { class: "desktop-column-contacts",
                        ul {
                            {
                                contacts_list.sort_by(|a, b| b.last_connection.cmp(&a.last_connection));
                                contacts_list
                                    .into_iter()
                                    .filter(|contact| {
                                        contact.name.to_lowercase().contains(&search_query().to_lowercase())
                                    })
                                    .map(|contact| {
                                        let topic_id = contact.id;
                                        let topic_name = contact.name;
                                        let avatar_url = contact.avatar_url;
                                        let last_message = contact.last_message;
                                        let last_connection = contact.last_connection;
                                        let id_for_chat = topic_id.clone();
                                        let id_for_details = topic_id.clone();
                                        let id_for_leave = topic_id.clone();
                                        let name_for_leave = topic_name.clone();
                                        rsx! {
                                            ContextMenu {
                                                ContextMenuTrigger {
                                                    TopicItem {
                                                        id: topic_id.clone(),
                                                        name: topic_name.clone(),
                                                        avatar_url,
                                                        last_message,
                                                        last_connection,
                                                        on_select: selected_topic_id,
                                                    }
                                                }
                                                ContextMenuContent { class: "context-menu-content",
                                                    ContextMenuItem {
                                                        class: "context-menu-item",
                                                        value: "Open Chat".to_string(),
                                                        index: 0usize,
                                                        on_select: move |_| {
                                                            selected_topic_id.set(Some(id_for_chat.clone()));
                                                        },
                                                        "Open Chat"
                                                    }
                                                    ContextMenuItem {
                                                        class: "context-menu-item",
                                                        value: "Open Details".to_string(),
                                                        index: 1usize,
                                                        on_select: move |_| {
                                                            let state = app_state.read();
                                                            if let Some(topic) = state.get_topic(&id_for_details) {
                                                                show_topic_details.set(Some(topic.clone()));
                                                            }
                                                        },
                                                        "Open Details"
                                                    }
                                                    ContextMenuItem {
                                                        class: "context-menu-item context-menu-item-danger",
                                                        value: "Leave Topic".to_string(),
                                                        index: 2usize,
                                                        on_select: {
                                                            move |_| {
                                                                show_leave_confirmation
                                                                    .set(Some((id_for_leave.clone(), name_for_leave.clone())))
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
                    }
                    div {
                        class: "profile-data",
                        onclick: move |_| {
                            show_profile_details.set(Some(profile_for_click.clone()));
                        },
                        img {
                            class: "profile-img",
                            src: "{avatar_url}",
                            alt: "Profile Avatar",
                        }
                        div { class: "profile-info",
                            h2 { class: "profile-name", "{profile_data.name}" }
                            p { class: "profile-status", "Online" }
                        }
                    }

                    if let Some(profile) = show_profile_details() {
                        ToastProvider {
                            ProfileDetails {
                                profile: profile.clone(),
                                toggle: show_profile_details,
                                on_modify_profile,
                                readonly: profile.id != profile_data.id,
                            }
                        }
                    }

                    if let Some(topic) = show_topic_details() {
                        ToastProvider {
                            TopicDetails {
                                topic: topic.clone(),
                                toggle: show_topic_details,
                                on_modify_topic,
                                view_profile: show_profile_details,
                                app_state,
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
                                selected_topic_id.set(None);
                            },
                        }
                    }
                }

                Chat {
                    app_state,
                    topic_id: selected_topic_id(),
                    on_send_message,
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
    fn TopicItem(
        id: String,
        name: String,
        avatar_url: Option<String>,
        last_message: Option<String>,
        last_connection: Option<u64>,
        on_select: Signal<Option<String>>,
    ) -> Element {
        let last_message_display = last_message.unwrap_or_default();

        let avatar_display = if let Some(url) = &avatar_url
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let time_display = if let Some(timestamp) = last_connection {
            format_relative_time(timestamp as i64)
        } else {
            String::from("")
        };

        rsx! {
            div {
                class: "desktop-contact-item",
                onclick: move |_| {
                    on_select.set(Some(id.clone()));
                },
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                img {
                    class: "desktop-contact-avatar",
                    src: "{avatar_display}",
                    alt: "{name}",
                    draggable: "false",
                }
                div { class: "desktop-contact-info",
                    h3 { class: "desktop-contact-name", "{name}" }
                    p { class: "desktop-contact-last-message", "{last_message_display}" }
                }
                h3 { class: "desktop-contact-last-connection", "{time_display}" }
            }
        }
    }

    #[component]
    fn Chat(
        app_state: Signal<AppState>,
        topic_id: Option<String>,
        on_send_message: EventHandler<(String, String)>,
    ) -> Element {
        let state = app_state.read();

        let topic = if let Some(id) = topic_id {
            state.get_topic(&id)
        } else {
            None
        };

        if let Some(topic) = topic {
            let messages = &topic.messages;
            let topic_name = &topic.name;

            let avatar_url = if let Some(url) = &topic.avatar_url {
                url.to_string()
            } else {
                DEFAULT_AVATAR.to_string()
            };

            let mut message_input = use_signal(String::new);

            let topic_id_str = topic.id.clone();
            let mut tracked_topic_id = use_signal(|| topic_id_str.clone());
            let mut last_msg_count = use_signal(|| 0);

            if *tracked_topic_id.read() != topic_id_str {
                tracked_topic_id.set(topic_id_str.clone());
                last_msg_count.set(0);
            }

            use_effect(move || {
                let state = app_state.read();
                let current_topic_id = tracked_topic_id.read();

                if let Some(t) = state.get_topic(&current_topic_id) {
                    let count = t.messages.len();
                    if count != *last_msg_count.read() {
                        last_msg_count.set(count);
                        document::eval(
                            r#"
                            requestAnimationFrame(() => {
                                const element = document.getElementById("chat-messages-container");
                                if (element) {
                                    element.scrollTop = element.scrollHeight;
                                }
                            });
                        "#,
                        );
                    }
                }
            });

            let send_message = use_callback({
                let topic_id = topic.id.clone();
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
                            src: "{avatar_url}",
                        }
                        h2 {
                            class: "desktop-contact-name",
                            title: "{topic_name}",
                            "{topic_name}"
                        }
                    }
                    div {
                        class: "desktop-chat-messages",
                        id: "chat-messages-container",
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
                            },
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
        } else {
            rsx! {
                div { class: "desktop-chat-placeholder",
                    h2 { "Select a topic to start chatting" }
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
        mut view_profile: Signal<Option<Profile>>,
        app_state: Signal<AppState>,
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
                                onchange: handle_image_change,
                            }
                        }
                        input {
                            class: "topic-details-title",
                            r#type: "text",
                            value: "{edited_title}",
                            oninput: move |e| edited_title.set(e.value()),
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
                    div { class: "topic-details-info-section",
                        p { class: "topic-details-section-title", "Members" }
                        ul {
                            {
                                let members: Vec<Profile> = {
                                    let state = app_state.read();
                                    let own_profile = state.get_profile();
                                    topic
                                        .members
                                        .iter()
                                        .map(|member_id| {
                                            if let Some(contact) = state.get_contact(member_id) {
                                                contact.clone()
                                            } else if member_id == &own_profile.id {
                                                own_profile.clone()
                                            } else {
                                                Profile::new_with_id(member_id)
                                            }
                                        })
                                        .collect()
                                };
                                members
                                    .into_iter()
                                    .map(|member| {
                                        let avatar = if let Some(url) = &member.avatar && !url.is_empty() {
                                            url.clone()
                                        } else {
                                            DEFAULT_AVATAR.to_string()
                                        };
                                        let last_seen = format_relative_time(
                                            (member.last_connection / 1000) as i64,
                                        );
                                        let member_clone = member.clone();
                                        rsx! {
                                            li {
                                                class: "topic-member-card",
                                                onclick: move |_| {
                                                    view_profile.set(Some(member_clone.clone()));
                                                },
                                                img { class: "topic-member-avatar", src: "{avatar}" }
                                                div { class: "topic-member-info",
                                                    h3 { class: "topic-member-name", "{member.name}" }
                                                    p { class: "topic-member-status", "{last_seen}" }
                                                }
                                            }
                                        }
                                    })
                            }
                        }
                    }
                }
            }
        }
    }

    #[component]
    fn ProfileDetails(
        profile: Profile,
        mut toggle: Signal<Option<Profile>>,
        on_modify_profile: EventHandler<Profile>,
        readonly: bool,
    ) -> Element {
        let toast = use_toast();
        let mut edited_name = use_signal(|| profile.name.clone());

        let handle_copy_profile_id = {
            let profile_id = profile.id.clone();
            move |_event: Event<MouseData>| match copy_to_clipboard(&profile_id) {
                Ok(_) => {
                    toast.success(
                        "Profile ID copied to clipboard!".to_owned(),
                        ToastOptions::default(),
                    );
                }
                Err(error) => {
                    toast.error(error, ToastOptions::default());
                }
            }
        };

        let profile_clone = profile.clone();
        let handle_save = move |_event: Event<MouseData>| {
            let mut updated_profile = profile_clone.clone();
            updated_profile.name = edited_name().trim().to_string();
            on_modify_profile.call(updated_profile);
            toast.success(
                "Profile updated successfully".to_owned(),
                ToastOptions::default(),
            );
            toggle.set(None);
        };

        let profile_clone_for_image = profile.clone();
        let handle_image_change = move |event: Event<FormData>| {
            let files = event.files();
            if let Some(file) = files.first() {
                let file = file.clone();
                let profile_clone = profile_clone_for_image.clone();
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
                            let url = format!(
                                "data:{};base64,{}",
                                file.content_type().unwrap_or("image/png".to_string()),
                                base64
                            );

                            let mut updated_profile = profile_clone.clone();
                            updated_profile.avatar = Some(url);
                            on_modify_profile.call(updated_profile);
                            toggle.set(None);

                            toast.success(
                                "Profile avatar updated successfully".to_owned(),
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

        let avatar_url = if let Some(url) = &profile.avatar
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let last_connection_text = if profile.last_connection > 0 {
            format_relative_time((profile.last_connection / 1000) as i64)
        } else {
            "Never".to_string()
        };

        rsx! {
            div {
                class: "profile-details-overlay",
                onclick: move |_| toggle.set(None),
                div {
                    class: "profile-details",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "profile-details-header",
                        label { class: "profile-details-image-wrapper",
                            img {
                                class: "profile-details-image",
                                src: avatar_url,
                            }
                            if !readonly {
                                input {
                                    r#type: "file",
                                    accept: "image/*",
                                    style: "display: none;",
                                    onchange: handle_image_change,
                                }
                            }
                        }
                        input {
                            class: "profile-details-name",
                            r#type: "text",
                            value: "{edited_name}",
                            placeholder: "Display Name",
                            readonly: "{readonly}",
                            oninput: move |e| edited_name.set(e.value()),
                        }
                        if !readonly {
                            button {
                                class: "profile-details-save-button",
                                onclick: handle_save,
                                "Save"
                            }
                        }
                    }
                    hr {}

                    div { class: "profile-details-info-section",
                        p { class: "profile-details-section-title", "Profile ID" }
                        p {
                            class: "profile-details-profile-id",
                            title: "Click to copy",
                            onclick: handle_copy_profile_id,
                            "{profile.id}"
                        }
                    }

                    div { class: "profile-details-info-section",
                        p { class: "profile-details-section-title", "Last Active" }
                        p { class: "profile-details-last-connection", "{last_connection_text}" }
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

    fn copy_to_clipboard(text: &str) -> Result<(), String> {
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text) {
                Ok(_) => Ok(()),
                Err(_) => Err("Failed to copy to clipboard".to_owned()),
            },
            Err(_) => Err("Failed to access clipboard".to_owned()),
        }
    }
}
