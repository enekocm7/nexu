pub mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::components::toast::ToastProvider;
    use crate::desktop::models::{
        AppState, ColumnState, ConnectionStatus, Message, Profile, ProfileChat, Topic,
        TopicCreationMode,
    };
    use arboard::Clipboard;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use chrono::{DateTime, Local, TimeDelta};
    use dioxus::html::FileData;
    use dioxus::prelude::*;
    use dioxus_primitives::context_menu::{
        ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
    };
    use dioxus_primitives::toast::{ToastOptions, Toasts, use_toast};

    #[derive(PartialEq, Clone, Copy, Debug)]
    pub enum RemovalType {
        Topic,
        Contact,
    }

    static DEFAULT_AVATAR: Asset = asset!("/assets/default_avatar.png");
    static CLOSE_ICON: Asset = asset!("/assets/close_icon.svg");
    static CLIP_ICON: Asset = asset!("/assets/clip_icon.svg");
    static COMPONENTS_CSS: Asset = asset!("/assets/dx-components-theme.css");

    #[component]
    pub fn Desktop(
        app_state: Signal<AppState>,
        on_create_topic: EventHandler<String>,
        on_join_topic: EventHandler<String>,
        on_leave_topic: EventHandler<String>,
        on_remove_contact: EventHandler<String>,
        on_send_message: EventHandler<(String, String)>,
        on_modify_topic: EventHandler<Topic>,
        on_modify_profile: EventHandler<Profile>,
        on_send_message_dm: EventHandler<(String, String)>,
        on_connect_peer: EventHandler<String>,
        on_add_contact: EventHandler<String>,
        on_image_send: EventHandler<(String, Vec<u8>)>,
    ) -> Element {
        let mut show_topic_dialog = use_signal(|| false);
        let mut show_contact_dialog = use_signal(|| false);
        let mut selected_topic_id = use_signal::<Option<String>>(|| None);
        let show_topic_details = use_signal::<Option<Topic>>(|| None);
        let mut show_profile_details = use_signal::<Option<Profile>>(|| None);
        let mut search_query = use_signal(String::new);
        let mut show_leave_confirmation =
            use_signal::<Option<(String, String, RemovalType)>>(|| None);
        let mut selected_column = use_signal::<ColumnState>(|| ColumnState::Contact);

        let profile_data: Profile = {
            let state = app_state();
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
            link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
            link { rel: "stylesheet", href: COMPONENTS_CSS }
            div {
                class: "font-[Segoe_UI,Tahoma,Geneva,Verdana,sans-serif] m-0 bg-bg-dark text-text-primary flex flex-row h-screen overflow-hidden",
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                div { class: "flex flex-col h-full bg-bg-panel w-[clamp(280px,25%,400px)] transition-[width] duration-300 ease-in-out relative border-r border-border",
                    div { class: "bg-bg-panel py-5 px-3.75 shadow-md",
                        div { class: "flex justify-between items-center mb-5",
                            h2 { class: "text-text-primary pl-1.25 m-0 text-[clamp(1.2rem,4vw,1.6rem)] font-semibold",
                                "Messages"
                            }
                            button {
                                class: "w-10 h-10 text-3xl leading-none bg-bg-subtle text-text-primary border-none rounded-xl flex items-center justify-center cursor-pointer transition-all duration-300 ease-in-out shadow-sm hover:bg-bg-active hover:-translate-y-0.5 hover:shadow-md active:translate-y-0 active:shadow-sm",
                                title: "New Topic",
                                title: if selected_column() == ColumnState::Topic { "New Topic" } else { "Add Contact" },
                                onclick: move |_| {
                                    if selected_column() == ColumnState::Topic {
                                        show_topic_dialog.set(true);
                                    } else {
                                        show_contact_dialog.set(true);
                                    }
                                },
                                "+"
                            }
                        }
                        input {
                            class: "input-field pl-11.25 bg-[url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='20' height='20' viewBox='0 0 24 24' fill='none' stroke='%23a1a1aa' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'%3E%3C/circle%3E%3Cpath d='m21 21-4.35-4.35'%3E%3C/path%3E%3C/svg%3E\")] bg-no-repeat bg-position-[15px_center] bg-size-[18px] focus:-translate-y-px",
                            r#type: "text",
                            icon: "search",
                            placeholder: "Search...",
                            oninput: move |value| {
                                search_query.set(value.value());
                            },
                        }
                        div { class: "grid grid-cols-2 mt-4 p-1 bg-bg-subtle rounded-lg gap-1",
                            button {
                                class: if selected_column() == ColumnState::Contact { "selected-column-button" } else { "unselected-topic-button" },
                                title: "Contacts",
                                onclick: move |_| selected_column.set(ColumnState::Contact),
                                "Contacts"
                            }
                            button {
                                class: if selected_column() == ColumnState::Topic { "selected-column-button" } else { "unselected-topic-button" },
                                title: "Topics",
                                onclick: move |_| selected_column.set(ColumnState::Topic),
                                "Topics"
                            }
                        }
                    }
                    div { class: "flex-1 overflow-y-auto overflow-x-hidden min-h-0 scrollbar-custom",
                        if selected_column() == ColumnState::Topic {
                            TopicColumn {
                                search_query,
                                selected_topic_id,
                                show_topic_details,
                                show_leave_confirmation,
                                app_state,
                            }
                        } else {
                            ContactColumn {
                                search_query,
                                selected_topic_id,
                                show_profile_details,
                                show_leave_confirmation,
                                app_state,
                            }
                        }
                    }
                    div {
                        class: "bg-bg-panel py-3 px-3.75 border-t border-[#444444] flex items-center gap-3 cursor-pointer transition-colors duration-200 mt-auto shrink-0 hover:bg-bg-hover active:bg-bg-active group",
                        onclick: move |_| {
                            show_profile_details.set(Some(profile_for_click.clone()));
                        },
                        img {
                            class: "avatar w-11.25 h-11.25 transition-colors duration-200 group-hover:border-accent",
                            src: "{avatar_url}",
                            alt: "Profile Avatar",
                        }
                        div { class: "flex-1 flex flex-col gap-1 overflow-hidden",
                            h2 { class: "m-0 text-white text-base font-semibold whitespace-nowrap overflow-hidden text-ellipsis",
                                "{profile_data.name}"
                            }
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


                    if show_contact_dialog() {
                        ContactDialog {
                            toggle: show_contact_dialog,
                            on_add: on_add_contact,
                        }
                    }

                    if let Some((id, name, removal_type)) = show_leave_confirmation() {
                        {
                            let (title, message, confirm_text) = match removal_type {

                                RemovalType::Topic => {
                                    (
                                        "Leave Topic".to_string(),
                                        format!(
                                            "Are you sure you want to leave \"{}\"? You will no longer receive messages from this topic.",
                                            name,
                                        ),
                                        "Leave".to_string(),
                                    )
                                }
                                RemovalType::Contact => {
                                    (
                                        "Remove Contact".to_string(),
                                        format!(
                                            "Are you sure you want to remove \"{}\" from your contacts?",
                                            name,
                                        ),
                                        "Remove".to_string(),
                                    )
                                }
                            };
                            rsx! {
                                ConfirmationDialog {
                                    title,
                                    message,
                                    confirm_text,
                                    cancel_text: "Cancel".to_string(),
                                    is_danger: true,
                                    toggle: show_leave_confirmation,
                                    on_confirm: move |_| {
                                        match removal_type {
                                            RemovalType::Topic => on_leave_topic.call(id.clone()),
                                            RemovalType::Contact => on_remove_contact.call(id.clone()),
                                        }
                                        show_leave_confirmation.set(None);
                                        selected_topic_id.set(None);
                                    },
                                }
                            }
                        }
                    }
                }

                Chat {
                    app_state,
                    topic_id: selected_topic_id(),
                    on_send_message,
                    on_send_message_dm,
                    on_image_send
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
                class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
                onclick: move |_| {
                    toggle.set(false);
                    topic_name.set(String::new());
                },
                div {
                    class: "card w-[90%] max-w-125 animate-[slideIn_0.3s_ease]",
                    onclick: move |e| {
                        e.stop_propagation();
                    },
                    div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                        h3 { class: "m-0 text-xl font-semibold text-text-primary",
                            "New Topic"
                        }
                        button {
                            class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "p-6",
                        div { class: "flex gap-2 mb-6 bg-bg-input p-1 rounded-[10px]",
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Create { "flex-1 py-2.5 px-4 bg-bg-subtle border-none rounded-lg text-text-primary text-sm font-medium cursor-pointer transition-all duration-200 shadow-sm" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-text-secondary text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-bg-hover hover:text-text-primary" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Create),
                                "Create Topic"
                            }
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Join { "flex-1 py-2.5 px-4 bg-bg-subtle border-none rounded-lg text-text-primary text-sm font-medium cursor-pointer transition-all duration-200 shadow-sm" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-text-secondary text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-bg-hover hover:text-text-primary" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Join),
                                "Join Topic"
                            }
                        }
                        div { class: "mb-5",
                            label { class: "block text-text-secondary text-sm font-medium mb-2",
                                if *selected_mode.read() == TopicCreationMode::Create {
                                    "Topic Name"
                                } else {
                                    "Topic ID or Invite Link"
                                }
                            }
                            input {
                                class: "input-field border-2 border-border focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                                r#type: "text",
                                value: "{topic_name}",
                                placeholder: if *selected_mode.read() == TopicCreationMode::Create { "Enter topic name..." } else { "Enter topic ID or paste invite link..." },
                                oninput: move |e| topic_name.set(e.value()),
                            }
                        }
                        p { class: "m-0 text-text-secondary text-[13px] leading-relaxed",
                            if *selected_mode.read() == TopicCreationMode::Create {
                                "Create a new topic to start chatting with others. You can share the topic ID with your friends."
                            } else {
                                "Join an existing topic by entering its ID or invite link shared by a friend."
                            }
                        }
                    }
                    div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                        button {
                            class: "btn-secondary py-2.5 px-6",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "btn-primary py-2.5 px-6 disabled:bg-bg-subtle disabled:text-text-muted disabled:cursor-not-allowed disabled:shadow-none",
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
    fn ContactDialog(mut toggle: Signal<bool>, on_add: EventHandler<String>) -> Element {
        let mut address_str = use_signal(String::new);

        let handle_submit = move |_| {
            let addr = address_str().trim().to_string();
            if !addr.is_empty() {
                on_add.call(addr);
                toggle.set(false);
                address_str.set(String::new());
            }
        };

        rsx! {
            div {
                class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
                onclick: move |_| {
                    toggle.set(false);
                    address_str.set(String::new());
                },
                div {
                    class: "card w-[90%] max-w-125 animate-[slideIn_0.3s_ease]",
                    onclick: move |e| {
                        e.stop_propagation();
                    },
                    div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                        h3 { class: "m-0 text-xl font-semibold text-text-primary",
                            "Add Contact"
                        }
                        button {
                            class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                            onclick: move |_| {
                                toggle.set(false);
                                address_str.set(String::new());
                            },
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "p-6",
                        div { class: "mb-5",
                            label { class: "block text-text-secondary text-sm font-medium mb-2",
                                "Contact Id"
                            }
                            input {
                                class: "input-field border-2 border-border focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                                r#type: "text",
                                value: "{address_str}",
                                placeholder: "Enter contact id...",
                                oninput: move |e| address_str.set(e.value()),
                            }
                        }
                        p { class: "m-0 text-text-secondary text-[13px] leading-relaxed",
                            "Enter the id of the user you want to add to your contacts."
                        }
                    }
                    div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                        button {
                            class: "btn-secondary py-2.5 px-6",
                            onclick: move |_| {
                                toggle.set(false);
                                address_str.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "btn-primary py-2.5 px-6 disabled:bg-bg-subtle disabled:text-text-muted disabled:cursor-not-allowed disabled:shadow-none",
                            disabled: address_str().trim().is_empty(),
                            onclick: handle_submit,
                            "Add Contact"
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
        mut toggle: Signal<Option<(String, String, RemovalType)>>,
        on_confirm: EventHandler<()>,
    ) -> Element {
        let button_class = if is_danger {
            "btn-danger py-2.5 px-6"
        } else {
            "btn-primary py-2.5 px-6"
        };

        rsx! {
            div {
                class: "fixed inset-0 bg-black/70 flex items-center justify-center z-1001 animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "card w-[90%] max-w-112.5 animate-[slideIn_0.3s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex justify-between items-center py-5 px-6 border-b border-border",
                        h3 { class: "m-0 text-xl font-semibold text-text-primary",
                            "{title}"
                        }
                        button {
                            class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-7500 [&:hover>img]:hue-rotate-324 [&:hover>img]:brightness-103 [&:hover>img]:contrast-103",
                            onclick: move |_| toggle.set(None),
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "p-6",
                        p { class: "m-0 text-text-secondary text-[15px] leading-relaxed",
                            "{message}"
                        }
                    }
                    div { class: "flex gap-3 justify-end py-5 px-6 border-t border-border bg-bg-input",
                        button {
                            class: "btn-secondary py-2.5 px-6",
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
    fn ColumnItem(
        id: String,
        name: String,
        avatar_url: Option<String>,
        last_message: Option<String>,
        last_connection: Option<u64>,
        on_select: Signal<Option<String>>,
        #[props(default)] highlight: Option<String>,
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

        let name_display = if let Some(query) = highlight.as_ref().filter(|q| !q.is_empty()) {
            let name_lower = name.to_lowercase();
            let query_lower = query.to_lowercase();
            if let Some(idx) = name_lower.find(&query_lower) {
                if name.len() == name_lower.len() {
                    let end = idx + query_lower.len();
                    let pre = &name[..idx];
                    let mat = &name[idx..end];
                    let post = &name[end..];
                    rsx! {
                        span {
                            "{pre}"
                            span { class: "text-accent", "{mat}" }
                            "{post}"
                        }
                    }
                } else {
                    rsx! { "{name}" }
                }
            } else {
                rsx! { "{name}" }
            }
        } else {
            rsx! { "{name}" }
        };

        rsx! {
            div {
                class: "list-item group",
                onclick: move |_| {
                    on_select.set(Some(id.clone()));
                },
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                img {
                    class: "avatar w-12.5 h-12.5 shrink-0 transition-colors duration-200 group-hover:border-text-muted",
                    src: "{avatar_display}",
                    alt: "{name}",
                    draggable: "false",
                }
                div { class: "flex-1 min-w-0 flex flex-col gap-1",
                    h3 { class: "m-0 text-[clamp(14px,2vw,16px)] font-semibold text-text-primary whitespace-nowrap overflow-hidden text-ellipsis",
                        {name_display}
                    }
                    p { class: "m-0 text-[clamp(12px,1.8vw,14px)] text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                        "{last_message_display}"
                    }
                }
                h3 { class: "m-0 text-[clamp(11px,1.5vw,12px)] font-normal text-text-muted shrink-0 self-start",
                    "{time_display}"
                }
            }
        }
    }

    #[component]
    fn Chat(
        app_state: Signal<AppState>,
        topic_id: Option<String>,
        on_send_message: EventHandler<(String, String)>,
        on_send_message_dm: EventHandler<(String, String)>,
        on_image_send: EventHandler<(String, Vec<u8>)>,
    ) -> Element {
        let state = app_state();

        let topic = if let Some(id) = &topic_id {
            state.get_topic(id)
        } else {
            None
        };

        let contact = if let Some(id) = &topic_id {
            state.get_contact_chat(id)
        } else {
            None
        };

        let (messages, title_text, avatar_url, chat_id) = if let Some(topic) = topic {
            (
                topic.messages.clone(),
                topic.name.clone(),
                topic
                    .avatar_url
                    .clone()
                    .unwrap_or(DEFAULT_AVATAR.to_string()),
                topic.id.clone(),
            )
        } else if let Some(contact) = contact {
            (
                contact
                    .messages
                    .iter()
                    .cloned()
                    .map(Message::from)
                    .collect::<Vec<Message>>(),
                contact.profile.name.clone(),
                contact
                    .profile
                    .avatar
                    .clone()
                    .unwrap_or(DEFAULT_AVATAR.to_string()),
                contact.profile.id.clone(),
            )
        } else {
            (
                Vec::<Message>::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        };

        if !chat_id.is_empty() {
            let mut message_input = use_signal(String::new);

            let mut tracked_id = use_signal(|| chat_id.clone());
            let mut last_msg_count = use_signal(|| 0);

            if *tracked_id.read() != chat_id {
                tracked_id.set(chat_id.clone());
                last_msg_count.set(0);
            }

            use_effect(move || {
                let state = app_state();
                let current_id = tracked_id.read();

                let count = if let Some(t) = state.get_topic(&current_id) {
                    t.messages.len()
                } else if let Some(c) = state.get_contact_chat(&current_id) {
                    c.messages.len()
                } else {
                    0
                };

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
            });

            let send_message = use_callback({
                let id = chat_id.clone();
                let is_dm = contact.is_some();
                move |_| {
                    let content = message_input().trim().to_string();
                    if !content.is_empty() {
                        if is_dm {
                            on_send_message_dm.call((id.clone(), content));
                        } else {
                            on_send_message.call((id.clone(), content));
                        }
                        message_input.set(String::new());
                    }
                }
            });

            let mut show_attachment = use_signal(|| false);

            let handle_media_submit = {
                move |files: Vec<FileData>| {
                    let chat_id = chat_id.clone();
                    let mut show_attachment = show_attachment;
                    spawn(async move {
                        for file in files {
                            if let Ok(data) = file.read_bytes().await {
                                on_image_send.call((chat_id.clone(), data.to_vec()));
                            }
                        }
                        show_attachment.set(false);
                    });
                }
            };

            rsx! {
                div { class: "flex-1 flex flex-col bg-bg-input h-full",
                    div { class: "bg-bg-panel py-3.75 px-5 shadow-md flex items-center gap-3.75 border-b border-border",
                        img {
                            class: "avatar w-11.25 h-11.25",
                            src: "{avatar_url}",
                        }
                        h2 {
                            class: "m-0 text-[clamp(1.1rem,2.5vw,1.4rem)] font-semibold text-text-primary max-w-100 overflow-hidden text-ellipsis whitespace-nowrap",
                            title: "{title_text}",
                            "{title_text}"
                        }
                    }
                    div {
                        class: "flex-1 overflow-y-auto p-5 flex flex-col gap-3 bg-bg-dark scrollbar-custom",
                        id: "chat-messages-container",
                        for message in messages.iter() {
                            ChatMessageComponent { message: message.clone(), app_state }
                        }
                    }
                    if show_attachment() {
                        AttachComponent {
                            on_select_media: handle_media_submit,
                            on_close: move |_| show_attachment.set(false),
                        }
                    }
                    div { class: "bg-bg-dark py-3.75 px-5 flex gap-3 items-center",
                        div { class: "flex-1 flex gap-2 items-center bg-bg-input rounded-lg border border-border px-4 py-2.5 transition-all duration-200 focus-within:border-accent-primary focus-within:shadow-[0_0_0_2px_rgba(59,130,246,0.2)]",
                            input {
                                class: "flex-1 bg-transparent border-none outline-none text-text-primary placeholder:text-text-secondary",
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
                                class: "btn-icon w-8 h-8 rounded-lg [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-73 [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-88 [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:saturate-7500",
                                onclick: move |_| show_attachment.set(true),
                                img { src: CLIP_ICON }
                            }
                        }
                        button {
                            class: "btn-primary py-3 px-6",
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
                div { class: "flex-1 flex items-center justify-center bg-bg-input text-text-secondary",
                    h2 { class: "text-[clamp(1.2rem,3vw,1.8rem)] font-medium m-0",
                        "Select a chat to start messaging"
                    }
                }
            }
        }
    }

    #[component]
    fn AttachComponent(
        on_select_media: EventHandler<Vec<FileData>>,
        on_close: EventHandler<()>,
    ) -> Element {
        rsx! {
            div {
                class: "fixed inset-0 z-40",
                onclick: move |_| on_close.call(()),
                div {
                    class: "absolute bottom-24 right-24 bg-bg-panel rounded-xl shadow-lg border border-border p-2 min-w-48 animate-[slideIn_0.2s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex flex-row gap-1",
                        label { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active group",
                            input {
                                class: "hidden",
                                r#type: "file",
                                multiple: true,
                                accept: "image/*,video/*",
                                onchange: move |e| on_select_media.call(e.files()),
                            }
                            p { class: "text-text-primary font-medium", "Photo/Video" }
                        }
                        button { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active text-left",
                            p { class: "text-text-primary font-medium", "Files" }
                        }
                        button { class: "flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all duration-200 hover:bg-bg-hover active:bg-bg-active text-left",
                            p { class: "text-text-primary font-medium", "Audio" }
                        }
                    }
                }
            }
        }
    }

    fn truncate_id(id: &str) -> String {
        if id.len() > 12 {
            let start = &id[..6];
            let end = &id[id.len() - 6..];
            format!("{}...{}", start, end)
        } else {
            id.to_string()
        }
    }

    fn get_sender_display_name(app_state: &AppState, sender_id: &str) -> String {
        let profile = app_state.get_profile();
        if sender_id == profile.id {
            return profile.name;
        }
        if let Some(contact) = app_state.get_contact(sender_id) {
            if contact.name == contact.id {
                truncate_id(sender_id)
            } else {
                contact.name.clone()
            }
        } else {
            truncate_id(sender_id)
        }
    }

    #[component]
    fn ChatMessageComponent(message: Message, app_state: Signal<AppState>) -> Element {
        let state = app_state();
        match message {
            Message::Chat(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                let sender_display = get_sender_display_name(&state, &message.sender_id);
                rsx! {
                    div { class: if message.is_sent { "message-bubble-sent" } else { "message-bubble-received" },
                        p {
                            class: "message-sender-id m-0 mb-1 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                            title: "{message.sender_id}",
                            "{sender_display}"
                        }
                        p { class: "m-0 max-w-96 text-[clamp(14px,2vw,15px)] leading-snug wrap-break-word",
                            "{message.content}"
                        }
                        p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 self-end",
                            "{timestamp_str}"
                        }
                    }
                }
            }
            Message::Leave(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                let sender_display = get_sender_display_name(&state, &message.sender_id);
                rsx! {
                    div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                            "{sender_display} has left the topic."
                        }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                            "{timestamp_str}"
                        }
                    }
                }
            }
            Message::Join(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                let sender_display = get_sender_display_name(&state, &message.sender_id);
                let text = if message.me {
                    format!("{sender_display} joined the topic.")
                } else {
                    format!("{sender_display} has joined the topic.")
                };

                rsx! {
                    div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                            "{text}"
                        }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                            "{timestamp_str}"
                        }
                    }
                }
            }
            Message::Disconnect(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                let sender_display = get_sender_display_name(&state, &message.sender_id);
                rsx! {
                    div { class: "max-w-full self-center bg-transparent text-text-muted py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-text-muted",
                            "{sender_display} has disconnected."
                        }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-text-muted",
                            "{timestamp_str}"
                        }
                    }
                }
            }
            Message::Image(message) => {
                let url = format!("data:image/png;base64,{}", message.image_url);
                let sender_display = get_sender_display_name(&state, &message.sender_id);
                let alignment = if message.is_sent {
                    "self-end"
                } else {
                    "self-start"
                };
                rsx! {
                    div { class: "max-w-[50%] flex flex-col gap-1 {alignment}",
                        if !message.is_sent {
                            p {
                                class: "m-0 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                                title: "{message.sender_id}",
                                "{sender_display}"
                            }
                        }
                        img {
                            class: "max-w-96 rounded-xl shadow-md",
                            src: "{url}",
                            alt: "Image message",
                        }
                        p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 text-text-secondary self-end",
                            "{format_message_timestamp(message.timestamp)}"
                        }
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
                Ok(clipboard) => copy_to_clipboard(clipboard, &topic_id, toast),
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
                            let processed_bytes = match process_image(&bytes) {
                                Ok(b) => b,
                                Err(e) => {
                                    toast.error(
                                        format!("Failed to process image: {}", e),
                                        ToastOptions::default(),
                                    );
                                    return;
                                }
                            };

                            const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                            if processed_bytes.len() > MAX_SIZE {
                                toast.error(
                                    "Image size must be less than 512 KB".to_owned(),
                                    ToastOptions::default(),
                                );
                                return;
                            }

                            let base64 = BASE64_STANDARD.encode(&processed_bytes);
                            let url = format!("data:image/webp;base64,{}", base64);

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
                class: "fixed inset-0 bg-black/60 flex items-center justify-center z-1000 animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "card w-[90%] max-w-125 p-6 animate-[slideIn_0.3s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex items-center gap-3 mb-5",
                        label { class: "cursor-pointer relative shrink-0 group",
                            img {
                                class: "avatar w-15 h-15 transition-all duration-200 group-hover:border-accent group-hover:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                                src: avatar_url,
                            }
                            input {
                                r#type: "file",
                                style: "display: none;",
                                onchange: handle_image_change,
                            }
                        }
                        input {
                            class: "input-field flex-1 m-0 text-2xl font-semibold border-2 border-border max-w-[90%] overflow-hidden text-ellipsis whitespace-nowrap focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            r#type: "text",
                            value: "{edited_title}",
                            oninput: move |e| edited_title.set(e.value()),
                        }
                        button {
                            class: "btn-primary py-3 px-6 whitespace-nowrap",
                            onclick: handle_save,
                            "Save"
                        }
                    }
                    hr { class: "border-none border-t border-border my-5" }
                    p { class: "my-4 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                        "Topic ID"
                    }
                    p {
                        class: "input-field m-0 border border-border font-mono text-sm break-all cursor-pointer hover:border-accent",
                        title: "Click to copy",
                        onclick: handle_copy_topic_id,
                        "{topic.id}"
                    }
                    div { class: "mb-4",
                        p { class: "my-4 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                            "Members"
                        }
                        ul {
                            {
                                let members: Vec<Profile> = {
                                    let state = app_state();
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

                                        let last_seen = match member.last_connection {
                                            ConnectionStatus::Online => member.last_connection.to_string(),
                                            ConnectionStatus::Offline(time) => {
                                                format_relative_time((time / 1000) as i64)
                                            }
                                        };
                                        let member_clone = member.clone();
                                        rsx! {
                                            li {
                                                class: "flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors duration-200 hover:bg-bg-hover list-none",
                                                onclick: move |_| {
                                                    view_profile.set(Some(member_clone.clone()));
                                                },
                                                img { class: "avatar w-10 h-10", src: "{avatar}" }
                                                div { class: "flex-1 flex flex-col gap-0.5 overflow-hidden",
                                                    h3 { class: "m-0 text-sm font-medium text-text-primary whitespace-nowrap overflow-hidden text-ellipsis",
                                                        "{member.name}"
                                                    }
                                                    p { class: "m-0 text-xs text-text-secondary whitespace-nowrap overflow-hidden text-ellipsis",
                                                        "{last_seen}"
                                                    }
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
        let mut edited_avatar = use_signal(|| profile.avatar.clone());

        let profile_id = profile.id.clone();
        let handle_copy_profile_id = {
            move |_event: Event<MouseData>| match Clipboard::new() {
                Ok(clipboard) => copy_to_clipboard(clipboard, &profile_id, toast),
                Err(_) => {
                    toast.error(
                        "Error accessing clipboard.".to_owned(),
                        ToastOptions::default(),
                    );
                }
            }
        };

        let profile_clone = profile.clone();
        let handle_save = move |_event: Event<MouseData>| {
            let mut updated_profile = profile_clone.clone();
            updated_profile.name = edited_name().trim().to_string();
            updated_profile.avatar = edited_avatar().clone();
            on_modify_profile.call(updated_profile);
            toast.success(
                "Profile updated successfully".to_owned(),
                ToastOptions::default(),
            );
            toggle.set(None);
        };

        let handle_image_change = move |event: Event<FormData>| {
            let files = event.files();
            if let Some(file) = files.first() {
                let file = file.clone();
                spawn(async move {
                    match file.read_bytes().await {
                        Ok(bytes) => {
                            let processed_bytes = match process_image(&bytes) {
                                Ok(b) => b,
                                Err(e) => {
                                    toast.error(
                                        format!("Failed to process image: {}", e),
                                        ToastOptions::default(),
                                    );
                                    return;
                                }
                            };

                            const MAX_SIZE: usize = 512 * 1024 * 4 / 3; // 512 KB
                            if processed_bytes.len() > MAX_SIZE {
                                toast.error(
                                    "Image size must be less than 512 KB".to_owned(),
                                    ToastOptions::default(),
                                );
                                return;
                            }

                            let base64 = BASE64_STANDARD.encode(&processed_bytes);
                            let url = format!("data:image/webp;base64,{}", base64);

                            edited_avatar.set(Some(url));

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

        let avatar_url = if let Some(url) = edited_avatar()
            && !url.is_empty()
        {
            url.clone()
        } else {
            DEFAULT_AVATAR.to_string()
        };

        let last_connection_text = match profile.last_connection {
            ConnectionStatus::Online => profile.last_connection.to_string(),
            ConnectionStatus::Offline(time) => format_relative_time((time / 1000) as i64),
        };

        rsx! {
            div {
                class: "fixed inset-0 bg-black/70 flex justify-center items-center z-2000 animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "card w-[90%] max-w-125 p-6 animate-[slideIn_0.3s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex items-center gap-3 mb-5",
                        label { class: "cursor-pointer relative shrink-0 group",
                            img {
                                class: "avatar w-15 h-15 transition-all duration-200 group-hover:border-accent group-hover:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                                src: avatar_url,
                            }
                            if !readonly {
                                input {
                                    r#type: "file",
                                    accept: "image/*",
                                    style: "display: none;",
                                    onchange: handle_image_change,
                                }
                                if profile.avatar.is_some_and(|url| !url.is_empty()) {
                                    button {
                                        class: "absolute -top-1 -right-1 w-5 h-5 rounded-full bg-danger hover:bg-danger-hover text-white text-xs font-bold flex items-center justify-center cursor-pointer transition-all duration-200 hover:scale-110 shadow-md pb-0.5 opacity-0 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto",
                                        title: "Remove Avatar",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            edited_avatar.set(None);
                                        },
                                        "✕"
                                    }
                                }
                            }
                        }
                        input {
                            class: "input-field flex-1 m-0 text-2xl font-semibold border-2 border-border min-w-0 focus:border-accent focus:shadow-[0_0_0_3px_rgba(59,130,246,0.2)]",
                            r#type: "text",
                            value: "{edited_name}",
                            placeholder: "Display Name",
                            readonly: "{readonly}",
                            oninput: move |e| edited_name.set(e.value()),
                        }
                        if !readonly {
                            button {
                                class: "btn-primary py-3 px-6 whitespace-nowrap",
                                onclick: handle_save,
                                "Save"
                            }
                        }
                    }
                    hr { class: "border-none border-t border-border my-5" }

                    div { class: "mb-4",
                        p { class: "m-0 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                            "Profile ID"
                        }
                        p {
                            class: "input-field m-0 border border-border font-mono text-sm break-all cursor-pointer hover:border-accent",
                            title: "Click to copy",
                            onclick: handle_copy_profile_id,
                            "{profile.id}"
                        }
                    }

                    div { class: "mb-0",
                        p { class: "m-0 mb-2 text-sm font-medium text-text-secondary uppercase tracking-wider",
                            "Last Active"
                        }
                        p { class: "input-field m-0 border border-border text-sm",
                            "{last_connection_text}"
                        }
                    }
                }
            }
        }
    }

    #[component]
    fn TopicColumn(
        search_query: Signal<String>,
        selected_topic_id: Signal<Option<String>>,
        show_topic_details: Signal<Option<Topic>>,
        show_leave_confirmation: Signal<Option<(String, String, RemovalType)>>,
        app_state: Signal<AppState>,
    ) -> Element {
        let topic_list: Vec<Topic> = {
            let state = app_state();
            let mut topics = state.get_all_topics().into_iter().collect::<Vec<Topic>>();
            topics.sort_by(|a, b| b.last_connection.cmp(&a.last_connection));
            topics
        };

        rsx! {
            ul {
                {
                    topic_list
                        .into_iter()
                        .filter(|topic| {
                            topic.name.to_lowercase().contains(&search_query().to_lowercase())
                                || topic.id.to_lowercase().contains(&search_query().to_lowercase())
                        })
                        .map(|topic| {
                            let topic_id = topic.id;
                            let topic_name = topic.name;
                            let avatar_url = topic.avatar_url;
                            let last_message = topic.last_message;
                            let last_connection = topic.last_connection;
                            let id_for_chat = topic_id.clone();
                            let id_for_details = topic_id.clone();
                            let id_for_leave = topic_id.clone();
                            let name_for_leave = topic_name.clone();
                            rsx! {
                                ContextMenu {
                                    ContextMenuTrigger {
                                        ColumnItem {
                                            id: topic_id.clone(),
                                            name: topic_name.clone(),
                                            avatar_url,
                                            last_message,
                                            last_connection,
                                            on_select: selected_topic_id,
                                            highlight: search_query(),
                                        }
                                    }
                                    ContextMenuContent { class: "context-menu",
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
                                                let state = app_state();
                                                if let Some(topic) = state.get_topic(&id_for_details) {
                                                    show_topic_details.set(Some(topic.clone()));
                                                }
                                            },
                                            "Open Details"
                                        }
                                        ContextMenuItem {
                                            class: "context-menu-item-danger",
                                            value: "Leave Topic".to_string(),
                                            index: 2usize,
                                            on_select: {
                                                move |_| {
                                                    show_leave_confirmation
                                                        .set(
                                                            Some((
                                                                id_for_leave.clone(),
                                                                name_for_leave.clone(),
                                                                RemovalType::Topic,
                                                            )),
                                                        )
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
    }

    #[component]
    fn ContactColumn(
        search_query: Signal<String>,
        selected_topic_id: Signal<Option<String>>,
        show_profile_details: Signal<Option<Profile>>,
        show_leave_confirmation: Signal<Option<(String, String, RemovalType)>>,
        app_state: Signal<AppState>,
    ) -> Element {
        let contact_list: Vec<ProfileChat> = {
            let state = app_state();
            let mut contacts = state.get_all_contacts_chat();
            contacts.sort_by(|a, b| b.profile.last_connection.cmp(&a.profile.last_connection));
            contacts
        };

        rsx! {
            ul {
                {
                    let filtered_contacts = contact_list
                        .into_iter()
                        .filter(|contact_chat| {
                            contact_chat
                                .profile
                                .name
                                .to_lowercase()
                                .contains(&search_query().to_lowercase())
                                || contact_chat
                                    .profile
                                    .id
                                    .to_lowercase()
                                    .contains(&search_query().to_lowercase())
                        })
                        .collect::<Vec<ProfileChat>>();
                    if filtered_contacts.is_empty() {
                        let message = if search_query().is_empty() {
                            "You have no contacts. Add some to start chatting!"
                        } else {
                            "No contacts found."
                        };
                        return rsx! {
                            div { class: "p-4 text-text-secondary text-center", "{message}" }
                        };
                    }
                    filtered_contacts
                        .into_iter()
                        .map(|contact_chat| {
                            let last_message = contact_chat.last_message();
                            let profile_id = contact_chat.profile.id;
                            let profile_name = contact_chat.profile.name;
                            let avatar_url = contact_chat.profile.avatar;
                            let last_connection = contact_chat.profile.last_connection.get_u64();
                            let id_for_chat = profile_id.clone();
                            let id_for_details = profile_id.clone();
                            let id_for_leave = profile_id.clone();
                            let name_for_leave = profile_name.clone();
                            rsx! {
                                ContextMenu {
                                    ContextMenuTrigger {
                                        ColumnItem {
                                            id: profile_id.clone(),
                                            name: profile_name.clone(),
                                            avatar_url,
                                            last_message,
                                            last_connection,
                                            on_select: selected_topic_id,
                                            highlight: search_query(),
                                        }
                                    }
                                    ContextMenuContent { class: "context-menu",
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
                                                let state = app_state();
                                                if let Some(contact) = state.get_contact(&id_for_details) {
                                                    show_profile_details.set(Some(contact.clone()));
                                                }
                                            },
                                            "Open Details"
                                        }
                                        ContextMenuItem {
                                            class: "context-menu-item-danger",
                                            value: "Leave Topic".to_string(),
                                            index: 2usize,
                                            on_select: {
                                                move |_| {
                                                    show_leave_confirmation
                                                        .set(
                                                            Some((
                                                                id_for_leave.clone(),
                                                                name_for_leave.clone(),
                                                                RemovalType::Contact,
                                                            )),
                                                        )
                                                }
                                            },
                                            "Remove Contact"
                                        }
                                    }
                                }
                            }
                        })
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

    fn copy_to_clipboard(mut clipboard: Clipboard, text: &str, toast: Toasts) {
        match clipboard.set_text(text) {
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
        }
    }

    fn process_image(file_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let image = image::load_from_memory(file_bytes)?;
        let resized = image.thumbnail(500, 500);
        let mut buffer = std::io::Cursor::new(Vec::new());
        resized.write_to(&mut buffer, image::ImageFormat::WebP)?;
        Ok(buffer.into_inner())
    }
}
