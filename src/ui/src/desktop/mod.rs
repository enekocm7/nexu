pub mod models;

#[cfg(feature = "desktop-web")]
pub mod desktop_web_components {
    use crate::components::toast::ToastProvider;
    use crate::desktop::models::{
        AppState, ConnectionStatus, Message, Profile, Topic, TopicCreationMode,
    };
    use arboard::Clipboard;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use chrono::{DateTime, Local, TimeDelta};
    use dioxus::prelude::*;
    use dioxus_primitives::context_menu::{
        ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
    };
    use dioxus_primitives::toast::{ToastOptions, use_toast};

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
            link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
            link { rel: "stylesheet", href: COMPONENTS_CSS }
            div {
                class: "font-[Segoe_UI,Tahoma,Geneva,Verdana,sans-serif] m-0 bg-[#212020] text-[#333333] flex flex-row h-screen overflow-hidden",
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                div { class: "flex flex-col h-full bg-[#444444] w-[clamp(280px,25%,400px)] transition-[width] duration-300 ease-in-out relative",
                    div { class: "bg-[#333333] py-5 px-[15px] shadow-[0_2px_8px_rgba(0,0,0,0.3)]",
                        div { class: "flex justify-between items-center mb-5",
                            h2 { class: "text-[ghostwhite] pl-[5px] m-0 text-[clamp(1.2rem,4vw,1.6rem)] font-semibold", "Messages" }
                            button {
                                class: "w-[45px] h-[45px] text-[28px] bg-[#4a4a4a] text-[whitesmoke] border-none rounded-xl flex items-center justify-center cursor-pointer transition-all duration-300 ease-in-out shadow-[0_2px_5px_rgba(0,0,0,0.2)] hover:bg-[#5a5a5a] hover:-translate-y-0.5 hover:shadow-[0_4px_8px_rgba(0,0,0,0.3)] active:translate-y-0 active:shadow-[0_2px_4px_rgba(0,0,0,0.2)]",
                                title: "New Topic",
                                onclick: move |_| show_topic_dialog.set(true),
                                "+"
                            }
                        }
                        input {
                            class: "w-full py-3 pr-4 pl-[45px] border-none rounded-xl bg-[#2a2a2a] bg-[url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='20' height='20' viewBox='0 0 24 24' fill='none' stroke='%23888888' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'%3E%3C/circle%3E%3Cpath d='m21 21-4.35-4.35'%3E%3C/path%3E%3C/svg%3E\")] bg-no-repeat bg-[position:15px_center] bg-[length:18px] text-white text-[15px] outline-none transition-all duration-300 ease-in-out shadow-[0_2px_5px_rgba(0,0,0,0.15)] box-border placeholder:text-[#888888] placeholder:font-normal focus:bg-[#353535] focus:shadow-[0_0_0_2px_rgba(100,150,255,0.4),0_4px_10px_rgba(0,0,0,0.25)] focus:-translate-y-px",
                            r#type: "text",
                            icon: "search",
                            placeholder: "Search",
                            oninput: move |value| {
                                search_query.set(value.value());
                            },
                        }
                    }
                    div { class: "flex-1 overflow-y-auto overflow-x-hidden min-h-0 [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-[#333333] [&::-webkit-scrollbar-thumb]:bg-[#666666] [&::-webkit-scrollbar-thumb]:rounded [&::-webkit-scrollbar-thumb:hover]:bg-[#777777]",
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
                                                ContextMenuContent { class: "bg-white border border-[#e0e0e0] rounded-lg shadow-[0_4px_12px_rgba(0,0,0,0.15)] p-1 min-w-[180px] z-[1000]",
                                                    ContextMenuItem {
                                                        class: "py-2 px-3 cursor-pointer rounded text-sm text-[#333333] transition-colors duration-200 hover:bg-[#f5f5f5] active:bg-[#eeeeee]",
                                                        value: "Open Chat".to_string(),
                                                        index: 0usize,
                                                        on_select: move |_| {
                                                            selected_topic_id.set(Some(id_for_chat.clone()));
                                                        },
                                                        "Open Chat"
                                                    }
                                                    ContextMenuItem {
                                                        class: "py-2 px-3 cursor-pointer rounded text-sm text-[#333333] transition-colors duration-200 hover:bg-[#f5f5f5] active:bg-[#eeeeee]",
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
                                                        class: "py-2 px-3 cursor-pointer rounded text-sm text-[#d9534f] transition-colors duration-200 hover:bg-[#f5f5f5] active:bg-[#eeeeee]",
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
                        class: "bg-[#333333] py-3 px-[15px] border-t border-[#444444] flex items-center gap-3 cursor-pointer transition-colors duration-200 mt-auto shrink-0 hover:bg-[#3a3a3a] active:bg-[#2a2a2a] group",
                        onclick: move |_| {
                            show_profile_details.set(Some(profile_for_click.clone()));
                        },
                        img {
                            class: "w-[45px] h-[45px] rounded-full object-cover border-2 border-[#555555] bg-[#666666] transition-colors duration-200 group-hover:border-[#5a7fb8]",
                            src: "{avatar_url}",
                            alt: "Profile Avatar",
                        }
                        div { class: "flex-1 flex flex-col gap-1 overflow-hidden",
                            h2 { class: "m-0 text-white text-base font-semibold whitespace-nowrap overflow-hidden text-ellipsis", "{profile_data.name}" }
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
                class: "fixed top-0 left-0 right-0 bottom-0 bg-black/60 flex items-center justify-center z-[1000] animate-[fadeIn_0.2s_ease]",
                onclick: move |_| {
                    toggle.set(false);
                    topic_name.set(String::new());
                },
                div {
                    class: "bg-[#333333] rounded-2xl w-[90%] max-w-[500px] shadow-[0_10px_40px_rgba(0,0,0,0.5)] animate-[slideIn_0.3s_ease] overflow-hidden",
                    onclick: move |e| {
                        e.stop_propagation();
                    },
                    div { class: "flex justify-between items-center py-5 px-6 border-b border-[#444444]",
                        h3 { class: "m-0 text-xl font-semibold text-white", "New Topic" }
                        button {
                            class: "bg-transparent border-none cursor-pointer p-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-200 leading-none hover:bg-[#444444] [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-[73%] [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-[88%] [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-[7500%] [&:hover>img]:hue-rotate-[324deg] [&:hover>img]:brightness-[103%] [&:hover>img]:contrast-[103%]",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "p-6",
                        div { class: "flex gap-2 mb-6 bg-[#2a2a2a] p-1 rounded-[10px]",
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Create { "flex-1 py-2.5 px-4 bg-[#4a4a4a] border-none rounded-lg text-white text-sm font-medium cursor-pointer transition-all duration-200 shadow-[0_2px_4px_rgba(0,0,0,0.2)]" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-[#aaaaaa] text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-[#353535] hover:text-white" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Create),
                                "Create Topic"
                            }
                            button {
                                class: if *selected_mode.read() == TopicCreationMode::Join { "flex-1 py-2.5 px-4 bg-[#4a4a4a] border-none rounded-lg text-white text-sm font-medium cursor-pointer transition-all duration-200 shadow-[0_2px_4px_rgba(0,0,0,0.2)]" } else { "flex-1 py-2.5 px-4 bg-transparent border-none rounded-lg text-[#aaaaaa] text-sm font-medium cursor-pointer transition-all duration-200 hover:bg-[#353535] hover:text-white" },
                                onclick: move |_| selected_mode.set(TopicCreationMode::Join),
                                "Join Topic"
                            }
                        }
                        div { class: "mb-5",
                            label { class: "block text-[#cccccc] text-sm font-medium mb-2",
                                if *selected_mode.read() == TopicCreationMode::Create {
                                    "Topic Name"
                                } else {
                                    "Topic ID or Invite Link"
                                }
                            }
                            input {
                                class: "w-full py-3 px-4 bg-[#2a2a2a] border-2 border-[#3a3a3a] rounded-[10px] text-white text-[15px] outline-none transition-all duration-200 box-border placeholder:text-[#777777] focus:border-[#5a7fb8] focus:bg-[#353535] focus:shadow-[0_0_0_3px_rgba(90,127,184,0.2)]",
                                r#type: "text",
                                value: "{topic_name}",
                                placeholder: if *selected_mode.read() == TopicCreationMode::Create { "Enter topic name..." } else { "Enter topic ID or paste invite link..." },
                                oninput: move |e| topic_name.set(e.value()),
                            }
                        }
                        p { class: "m-0 text-[#999999] text-[13px] leading-relaxed",
                            if *selected_mode.read() == TopicCreationMode::Create {
                                "Create a new topic to start chatting with others. You can share the topic ID with your friends."
                            } else {
                                "Join an existing topic by entering its ID or invite link shared by a friend."
                            }
                        }
                    }
                    div { class: "flex gap-3 justify-end py-5 px-6 border-t border-[#444444] bg-[#2a2a2a]",
                        button {
                            class: "py-2.5 px-6 border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 bg-[#3a3a3a] text-[#cccccc] hover:bg-[#454545] hover:text-white",
                            onclick: move |_| {
                                toggle.set(false);
                                topic_name.set(String::new());
                            },
                            "Cancel"
                        }
                        button {
                            class: "py-2.5 px-6 border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 bg-[#5a7fb8] text-white hover:bg-[#6a8fc8] hover:shadow-[0_4px_12px_rgba(90,127,184,0.3)] disabled:bg-[#3a3a3a] disabled:text-[#666666] disabled:cursor-not-allowed disabled:shadow-none",
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
            "py-2.5 px-6 border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 bg-[#d9534f] text-white hover:bg-[#e76460] hover:shadow-[0_4px_12px_rgba(217,83,79,0.4)]"
        } else {
            "py-2.5 px-6 border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 bg-[#5a7fb8] text-white hover:bg-[#6a8fc8] hover:shadow-[0_4px_12px_rgba(90,127,184,0.3)]"
        };

        rsx! {
            div {
                class: "fixed top-0 left-0 right-0 bottom-0 bg-black/70 flex items-center justify-center z-[1001] animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "bg-[#333333] rounded-2xl w-[90%] max-w-[450px] shadow-[0_10px_40px_rgba(0,0,0,0.6)] animate-[slideIn_0.3s_ease] overflow-hidden",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex justify-between items-center py-5 px-6 border-b border-[#444444]",
                        h3 { class: "m-0 text-xl font-semibold text-white", "{title}" }
                        button {
                            class: "bg-transparent border-none cursor-pointer p-0 w-8 h-8 flex items-center justify-center rounded-lg transition-all duration-200 hover:bg-[#444444] [&>img]:w-5 [&>img]:h-5 [&>img]:brightness-0 [&>img]:saturate-100 [&>img]:invert-[73%] [&>img]:sepia-0 [&>img]:hue-rotate-180 [&>img]:contrast-[88%] [&>img]:transition-[filter] [&>img]:duration-200 [&:hover>img]:invert-100 [&:hover>img]:sepia-0 [&:hover>img]:saturate-[7500%] [&:hover>img]:hue-rotate-[324deg] [&:hover>img]:brightness-[103%] [&:hover>img]:contrast-[103%]",
                            onclick: move |_| toggle.set(None),
                            img { src: CLOSE_ICON }
                        }
                    }
                    div { class: "p-6",
                        p { class: "m-0 text-[#cccccc] text-[15px] leading-relaxed", "{message}" }
                    }
                    div { class: "flex gap-3 justify-end py-5 px-6 border-t border-[#444444] bg-[#2a2a2a]",
                        button {
                            class: "py-2.5 px-6 border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 bg-[#3a3a3a] text-[#cccccc] hover:bg-[#454545] hover:text-white",
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
                class: "flex items-center py-3 px-[15px] gap-3 bg-[#444444] border-b border-[#3a3a3a] cursor-pointer transition-all duration-200 hover:bg-[#505050] hover:translate-x-[3px] active:bg-[#4a4a4a] group",
                onclick: move |_| {
                    on_select.set(Some(id.clone()));
                },
                oncontextmenu: move |e| {
                    e.prevent_default();
                },
                img {
                    class: "w-[50px] h-[50px] rounded-full object-cover shrink-0 bg-[#666666] border-2 border-[#555555] transition-colors duration-200 group-hover:border-[#777777]",
                    src: "{avatar_display}",
                    alt: "{name}",
                    draggable: "false",
                }
                div { class: "flex-1 min-w-0 flex flex-col gap-1",
                    h3 { class: "m-0 text-[clamp(14px,2vw,16px)] font-semibold text-white whitespace-nowrap overflow-hidden text-ellipsis", "{name}" }
                    p { class: "m-0 text-[clamp(12px,1.8vw,14px)] text-[#aaaaaa] whitespace-nowrap overflow-hidden text-ellipsis", "{last_message_display}" }
                }
                h3 { class: "m-0 text-[clamp(11px,1.5vw,12px)] font-normal text-[#888888] shrink-0 self-start", "{time_display}" }
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
                div { class: "flex-1 flex flex-col bg-[#2a2a2a] h-full",
                    div { class: "bg-[#333333] py-[15px] px-5 shadow-[0_2px_8px_rgba(0,0,0,0.3)] flex items-center gap-[15px] border-b border-[#3a3a3a]",
                        img {
                            class: "w-[45px] h-[45px] rounded-full object-cover bg-[#666666] border-2 border-[#555555]",
                            src: "{avatar_url}",
                        }
                        h2 {
                            class: "m-0 text-[clamp(1.1rem,2.5vw,1.4rem)] font-semibold text-white max-w-[400px] overflow-hidden text-ellipsis whitespace-nowrap",
                            title: "{topic_name}",
                            "{topic_name}"
                        }
                    }
                    div {
                        class: "flex-1 overflow-y-auto p-5 flex flex-col gap-3 bg-[#212020] [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar-track]:bg-[#1a1a1a] [&::-webkit-scrollbar-thumb]:bg-[#4a4a4a] [&::-webkit-scrollbar-thumb]:rounded [&::-webkit-scrollbar-thumb:hover]:bg-[#5a5a5a]",
                        id: "chat-messages-container",
                        for message in messages.iter() {
                            ChatMessageComponent { message: message.clone() }
                        }
                    }
                    div { class: "bg-[#212020] py-[15px] px-5 flex gap-3 items-center",
                        input {
                            class: "flex-1 py-3 px-4 border-none rounded-xl bg-[#2a2a2a] text-white text-[15px] outline-none transition-all duration-300 ease-in-out shadow-[0_2px_5px_rgba(0,0,0,0.15)] placeholder:text-[#888888] focus:bg-[#353535] focus:shadow-[0_0_0_2px_rgba(100,150,255,0.4),0_4px_10px_rgba(0,0,0,0.25)]",
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
                            class: "py-3 px-6 bg-[#5a7fb8] text-white border-none rounded-xl text-[15px] font-medium cursor-pointer transition-all duration-300 ease-in-out shadow-[0_2px_5px_rgba(0,0,0,0.2)] hover:bg-[#6a8fc8] hover:-translate-y-0.5 hover:shadow-[0_4px_8px_rgba(90,127,184,0.3)] active:translate-y-0 active:shadow-[0_2px_4px_rgba(0,0,0,0.2)]",
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
                div { class: "flex-1 flex items-center justify-center bg-[#2a2a2a] text-[#888888]",
                    h2 { class: "text-[clamp(1.2rem,3vw,1.8rem)] font-medium m-0", "Select a topic to start chatting" }
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
                    div { class: if message.is_sent { "max-w-[70%] py-2.5 px-3.5 rounded-xl break-words flex flex-col gap-1 self-end bg-[#5a7fb8] text-white [&>.message-sender-id]:hidden" } else { "max-w-[70%] py-2.5 px-3.5 rounded-xl break-words flex flex-col gap-1 self-start bg-[#3a3a3a] text-white [&>.message-sender-id]:text-left [&>.message-sender-id]:text-[#c0c0c0]" },
                        p { class: "message-sender-id m-0 mb-1 text-[clamp(11px,1.6vw,12px)] font-medium opacity-80 text-[#e0e0e0] whitespace-nowrap overflow-hidden text-ellipsis", "{message.sender_id}" }
                        p { class: "m-0 text-[clamp(14px,2vw,15px)] leading-snug", "{message.content}" }
                        p { class: "m-0 text-[clamp(10px,1.5vw,11px)] opacity-70 self-end", "{timestamp_str}" }
                    }
                }
            }
            Message::Leave(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                rsx! {
                    div { class: "max-w-full self-center bg-transparent text-[#888888] py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-[#888888]", "{message.sender_id} has left the topic." }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-[#777777]", "{timestamp_str}" }
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
                    div { class: "max-w-full self-center bg-transparent text-[#888888] py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-[#888888]", "{text}" }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-[#777777]", "{timestamp_str}" }
                    }
                }
            }
            Message::Disconnect(message) => {
                let timestamp_str = format_message_timestamp(message.timestamp);
                rsx! {
                    div { class: "max-w-full self-center bg-transparent text-[#888888] py-2 px-3 text-[clamp(12px,1.8vw,13px)] italic text-center",
                        p { class: "m-0 text-[clamp(12px,1.8vw,13px)] opacity-85 text-[#888888]", "{message.sender_id} has disconnected." }
                        p { class: "mt-1 mb-0 text-[clamp(10px,1.5vw,11px)] opacity-60 text-[#777777]", "{timestamp_str}" }
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
                class: "fixed top-0 left-0 right-0 bottom-0 bg-black/60 flex items-center justify-center z-[1000] animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "bg-[#333333] rounded-2xl w-[90%] max-w-[500px] p-6 shadow-[0_10px_40px_rgba(0,0,0,0.5)] animate-[slideIn_0.3s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex items-center gap-3 mb-5",
                        label { class: "cursor-pointer relative shrink-0 group",
                            img { class: "w-[60px] h-[60px] rounded-full object-cover bg-[#666666] border-2 border-[#555555] transition-all duration-200 group-hover:border-[#5a7fb8] group-hover:shadow-[0_0_0_3px_rgba(90,127,184,0.2)]", src: avatar_url }
                            input {
                                r#type: "file",
                                style: "display: none;",
                                onchange: handle_image_change,
                            }
                        }
                        input {
                            class: "flex-1 m-0 py-3 px-4 text-2xl font-semibold text-white bg-[#2a2a2a] border-2 border-[#3a3a3a] rounded-[10px] outline-none transition-all duration-200 min-w-0 max-w-[90%] overflow-hidden text-ellipsis whitespace-nowrap focus:border-[#5a7fb8] focus:bg-[#353535] focus:shadow-[0_0_0_3px_rgba(90,127,184,0.2)]",
                            r#type: "text",
                            value: "{edited_title}",
                            oninput: move |e| edited_title.set(e.value()),
                        }
                        button {
                            class: "py-3 px-6 bg-[#5a7fb8] text-white border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 whitespace-nowrap hover:bg-[#6a8fc8] hover:shadow-[0_4px_12px_rgba(90,127,184,0.3)] active:translate-y-px",
                            onclick: handle_save,
                            "Save"
                        }
                    }
                    hr { class: "border-none border-t border-[#444444] my-5" }
                    p { class: "my-4 mb-2 text-sm font-medium text-[#aaaaaa] uppercase tracking-wider", "Topic ID" }
                    p {
                        class: "m-0 py-3 px-4 bg-[#2a2a2a] border border-[#3a3a3a] rounded-lg text-white font-mono text-sm break-all cursor-pointer transition-all duration-200 hover:bg-[#353535] hover:border-[#5a7fb8]",
                        title: "Click to copy",
                        onclick: handle_copy_topic_id,
                        "{topic.id}"
                    }
                    div { class: "mb-4",
                        p { class: "my-4 mb-2 text-sm font-medium text-[#aaaaaa] uppercase tracking-wider", "Members" }
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

                                        let last_seen = match member.last_connection {
                                            ConnectionStatus::Online => member.last_connection.to_string(),
                                            ConnectionStatus::Offline(time) => {
                                                format_relative_time((time / 1000) as i64)
                                            }
                                        };
                                        let member_clone = member.clone();
                                        rsx! {
                                            li {
                                                class: "flex items-center gap-3 p-3 rounded-lg cursor-pointer transition-colors duration-200 hover:bg-[#3a3a3a] list-none",
                                                onclick: move |_| {
                                                    view_profile.set(Some(member_clone.clone()));
                                                },
                                                img { class: "w-10 h-10 rounded-full object-cover bg-[#666666] border-2 border-[#555555]", src: "{avatar}" }
                                                div { class: "flex-1 flex flex-col gap-0.5 overflow-hidden",
                                                    h3 { class: "m-0 text-sm font-medium text-white whitespace-nowrap overflow-hidden text-ellipsis", "{member.name}" }
                                                    p { class: "m-0 text-xs text-[#aaaaaa] whitespace-nowrap overflow-hidden text-ellipsis", "{last_seen}" }
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

        let last_connection_text = match profile.last_connection {
            ConnectionStatus::Online => profile.last_connection.to_string(),
            ConnectionStatus::Offline(time) => format_relative_time((time / 1000) as i64),
        };

        rsx! {
            div {
                class: "fixed top-0 left-0 right-0 bottom-0 bg-black/70 flex justify-center items-center z-[2000] animate-[fadeIn_0.2s_ease]",
                onclick: move |_| toggle.set(None),
                div {
                    class: "bg-[#333333] rounded-2xl w-[90%] max-w-[500px] p-6 shadow-[0_10px_40px_rgba(0,0,0,0.5)] animate-[slideIn_0.3s_ease]",
                    onclick: move |e| e.stop_propagation(),
                    div { class: "flex items-center gap-3 mb-5",
                        label { class: "cursor-pointer relative shrink-0 group",
                            img {
                                class: "w-[60px] h-[60px] rounded-full object-cover bg-[#666666] border-2 border-[#555555] transition-all duration-200 group-hover:border-[#5a7fb8] group-hover:shadow-[0_0_0_3px_rgba(90,127,184,0.2)]",
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
                            class: "flex-1 m-0 py-3 px-4 text-2xl font-semibold text-white bg-[#2a2a2a] border-2 border-[#3a3a3a] rounded-[10px] outline-none transition-all duration-200 min-w-0 focus:border-[#5a7fb8] focus:bg-[#353535] focus:shadow-[0_0_0_3px_rgba(90,127,184,0.2)]",
                            r#type: "text",
                            value: "{edited_name}",
                            placeholder: "Display Name",
                            readonly: "{readonly}",
                            oninput: move |e| edited_name.set(e.value()),
                        }
                        if !readonly {
                            button {
                                class: "py-3 px-6 bg-[#5a7fb8] text-white border-none rounded-[10px] text-[15px] font-medium cursor-pointer transition-all duration-200 whitespace-nowrap hover:bg-[#6a8fc8] hover:shadow-[0_4px_12px_rgba(90,127,184,0.3)] active:translate-y-px",
                                onclick: handle_save,
                                "Save"
                            }
                        }
                    }
                    hr { class: "border-none border-t border-[#444444] my-5" }

                    div { class: "mb-4",
                        p { class: "m-0 mb-2 text-sm font-medium text-[#aaaaaa] uppercase tracking-wider", "Profile ID" }
                        p {
                            class: "m-0 py-3 px-4 bg-[#2a2a2a] border border-[#3a3a3a] rounded-lg text-white font-mono text-sm break-all cursor-pointer transition-all duration-200 hover:bg-[#353535] hover:border-[#5a7fb8]",
                            title: "Click to copy",
                            onclick: handle_copy_profile_id,
                            "{profile.id}"
                        }
                    }

                    div { class: "mb-0",
                        p { class: "m-0 mb-2 text-sm font-medium text-[#aaaaaa] uppercase tracking-wider", "Last Active" }
                        p { class: "m-0 py-3 px-4 bg-[#2a2a2a] border border-[#3a3a3a] rounded-lg text-white text-sm", "{last_connection_text}" }
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
